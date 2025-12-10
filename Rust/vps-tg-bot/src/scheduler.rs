use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use crate::system::SystemOps;
use crate::error::SchedulerError;
use crate::config::Config;
use log::{info, error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobType {
    CoreMaintain,
    RulesUpdate,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub job_type: JobType,
    #[serde(with = "cron_serde")]
    pub schedule: Schedule,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
}

mod cron_serde {
    use super::*;
    use std::str::FromStr;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(schedule: &Schedule, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&schedule.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Schedule, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Schedule::from_str(&s).map_err(serde::de::Error::custom)
    }
}

pub struct Scheduler {
    pub jobs: HashMap<JobType, ScheduledJob>,
    pub system: Arc<dyn SystemOps>,
    state_path: PathBuf,
    jobs_mutex: Arc<Mutex<()>>, // 用于文件操作的互斥锁
}

#[derive(Debug)]
pub enum SchedulerCommand {
    StartJob(JobType),
    StopJob(JobType),
    UpdateSchedule(JobType, String),
    ForceRun(JobType),
}

impl Scheduler {
    pub fn new(config: &Config, system: Arc<dyn SystemOps>) -> Result<Self, SchedulerError> {
        let mut scheduler = Self {
            jobs: HashMap::new(),
            system,
            state_path: config.state_path.join("jobs.json"),
            jobs_mutex: Arc::new(Mutex::new(())),
        };
        
        scheduler.load_jobs()?;
        Ok(scheduler)
    }

    pub fn add_job(&mut self, job: ScheduledJob) -> Result<(), SchedulerError> {
        self.jobs.insert(job.job_type, job);
        self.save_jobs()
    }

    pub fn remove_job(&mut self, job_type: JobType) -> Result<Option<ScheduledJob>, SchedulerError> {
        let job = self.jobs.remove(&job_type);
        self.save_jobs()?;
        Ok(job)
    }

    fn save_jobs(&self) -> Result<(), SchedulerError> {
        let _guard = self.jobs_mutex.lock().map_err(|_| SchedulerError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Mutex poisoned")))?;
        
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SchedulerError::IoError(e))?;
        }
        
        let json = serde_json::to_string_pretty(&self.jobs)
            .map_err(|e| SchedulerError::SerializationError(e))?;
        
        // 使用原子操作写入临时文件后重命名
        let temp_path = self.state_path.with_extension("tmp");
        fs::write(&temp_path, json).map_err(|e| SchedulerError::IoError(e))?;
        fs::rename(&temp_path, &self.state_path).map_err(|e| SchedulerError::IoError(e))?;
        
        Ok(())
    }

    fn load_jobs(&mut self) -> Result<(), SchedulerError> {
        let _guard = self.jobs_mutex.lock().map_err(|_| SchedulerError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Mutex poisoned")))?;
        
        if !self.state_path.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.state_path).map_err(|e| SchedulerError::IoError(e))?;
        self.jobs = serde_json::from_str(&content).map_err(|e| SchedulerError::SerializationError(e))?;
        Ok(())
    }

    pub fn run(&mut self, rx: mpsc::Receiver<SchedulerCommand>) {
        info!("Scheduler started");
        
        loop {
            if let Ok(cmd) = rx.try_recv() {
                self.handle_command(cmd);
            }

            let now = Utc::now();
            let mut jobs_to_run = Vec::new();

            for (job_type, job) in self.jobs.iter() {
                if !job.enabled {
                    continue;
                }

                if let Some(next_run) = job.schedule.upcoming(Utc).next() {
                    let should_run = match job.last_run {
                        Some(last) => {
                             if let Some(next) = job.schedule.after(&last).next() {
                                 next <= now
                             } else {
                                 false
                             }
                        },
                        None => {
                            // If never run, set last_run to now but don't run immediately to avoid storm
                            // Or should we? Let's say we mark it as run.
                            // However, we can't mutate job here because we are iterating.
                            // We will collect jobs to modify/run.
                            false
                        }
                    };
                    
                    if should_run {
                        jobs_to_run.push(*job_type);
                    }
                }
            }
            
            // Handle first time initialization logic separately if needed, 
            // but here we just handle jobs that need running.
            // Also need to handle 'None' case for last_run to initialize it.
            for (_, job) in self.jobs.iter_mut() {
                if job.last_run.is_none() {
                     job.last_run = Some(now);
                     // We don't save every time to avoid IO spam, but for initialization we should.
                     // We will save at the end of loop iteration if changes happened.
                }
            }

            // Iterate over a copy/clone of the list to avoid move
            for job_type in &jobs_to_run {
                info!("Executing scheduled job: {:?}", job_type);
                self.execute_job(*job_type);
                if let Some(job) = self.jobs.get_mut(job_type) {
                     job.last_run = Some(now);
                }
            }
            
            if !jobs_to_run.is_empty() {
                if let Err(e) = self.save_jobs() {
                    error!("Failed to save jobs after running scheduled jobs: {}", e);
                }
            }

            std::thread::sleep(Duration::from_secs(10));
        }
    }

    fn handle_command(&mut self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::StartJob(t) => {
                if let Some(job) = self.jobs.get_mut(&t) {
                    job.enabled = true;
                    if let Err(e) = self.save_jobs() {
                        error!("Failed to save jobs after enabling job {:?}: {}", t, e);
                    }
                }
            },
            SchedulerCommand::StopJob(t) => {
                if let Some(job) = self.jobs.get_mut(&t) {
                    job.enabled = false;
                    if let Err(e) = self.save_jobs() {
                        error!("Failed to save jobs after disabling job {:?}: {}", t, e);
                    }
                }
            },
            SchedulerCommand::UpdateSchedule(t, s) => {
                if let Ok(schedule) = s.parse::<Schedule>() {
                    if let Some(job) = self.jobs.get_mut(&t) {
                        job.schedule = schedule;
                        if let Err(e) = self.save_jobs() {
                            error!("Failed to save jobs after updating schedule for job {:?}: {}", t, e);
                        }
                    }
                }
            },
            SchedulerCommand::ForceRun(t) => {
                self.execute_job(t);
            }
        }
    }

    fn execute_job(&self, job_type: JobType) {
        let (script, timeout_secs) = match job_type {
            JobType::CoreMaintain => ("/usr/local/bin/vps-maintain-core.sh", 300),
            JobType::RulesUpdate => ("/usr/local/bin/vps-maintain-rules.sh", 120),
        };

        match self.system.execute_script(script, Duration::from_secs(timeout_secs)) {
            Ok(res) => {
                info!("Job {:?} finished: success={}", job_type, res.success);
            },
            Err(e) => {
                error!("Job {:?} failed: {}", job_type, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::MockSystemOps;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &TempDir) -> Config {
        Config {
            tg_token: "token".to_string(),
            tg_chat_id: 123456,
            state_path: temp_dir.path().to_path_buf(),
            scripts_path: temp_dir.path().to_path_buf(),
            logs_service: "service".to_string(),
        }
    }

    #[test]
    fn test_add_remove_job() {
        let temp = TempDir::new().unwrap();
        let system = Arc::new(MockSystemOps::new());
        let config = create_test_config(&temp);
        let mut scheduler = Scheduler::new(&config, system).unwrap();

        let job = ScheduledJob {
            job_type: JobType::CoreMaintain,
            schedule: Schedule::from_str("0 0 * * * *").unwrap(),
            enabled: true,
            last_run: None,
        };

        scheduler.add_job(job);
        assert!(scheduler.jobs.contains_key(&JobType::CoreMaintain));

        let content = fs::read_to_string(temp.path().join("jobs.json")).unwrap();
        assert!(content.contains("CoreMaintain"));

        let removed = scheduler.remove_job(JobType::CoreMaintain);
        assert!(removed.is_some());
        assert!(!scheduler.jobs.contains_key(&JobType::CoreMaintain));
    }
    
    #[test]
    fn test_load_jobs() {
        let temp = TempDir::new().unwrap();
        let system = Arc::new(MockSystemOps::new());
        let config = create_test_config(&temp);
        
        let jobs_path = temp.path().join("jobs.json");
        let initial_json = r#"{
            "CoreMaintain": {
                "job_type": "CoreMaintain",
                "schedule": "0 0 4 * * * *",
                "enabled": true,
                "last_run": null
            }
        }"#;
        fs::write(jobs_path, initial_json).unwrap();

        let scheduler = Scheduler::new(&config, system).unwrap();
        assert!(scheduler.jobs.contains_key(&JobType::CoreMaintain));
        let job = scheduler.jobs.get(&JobType::CoreMaintain).unwrap();
        assert_eq!(job.enabled, true);
    }
}
