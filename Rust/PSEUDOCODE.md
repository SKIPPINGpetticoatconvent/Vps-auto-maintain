# VPS Telegram Bot (Rust Port) Pseudocode

## 1. Main Entry Point (`main.rs`)

```rust
// TDD Anchor: Test that application panics if required env vars are missing
fn main() {
    // 1. Load configuration from Environment Variables
    let config = Config::from_env().expect("Failed to load configuration");
    
    // 2. Initialize Logger (env_logger)
    init_logger();
    
    // 3. Initialize Scheduler
    let scheduler = Scheduler::new(config.state_path);
    // TDD Anchor: Test that scheduler loads persisted jobs correctly
    scheduler.load_state();
    
    // 4. Initialize Bot
    let bot = Bot::new(config.tg_token);
    
    // 5. Start Scheduler in a separate thread
    let scheduler_handle = thread::spawn(move || {
        scheduler.run();
    });
    
    // 6. Start Bot Polling (Blocking)
    // TDD Anchor: Test that bot handles updates and filters by chat_id
    bot.start_polling(config.tg_chat_id, scheduler_handle);
}
```

## 2. Configuration Module (`config.rs`)

```rust
struct Config {
    tg_token: String,
    tg_chat_id: i64,
    state_path: PathBuf,
}

impl Config {
    fn from_env() -> Result<Self, ConfigError> {
        // Get TG_TOKEN, return error if missing
        // Get TG_CHAT_ID, parse to i64, return error if missing or invalid
        // Default state_path to "/etc/vps-tg-bot/state.json" or env override
    }
}
```

## 3. Bot Module (`bot.rs`)

```rust
struct Bot {
    token: String,
    api: TelegramApi, // Wrapper around reqwest/teloxide
}

impl Bot {
    fn start_polling(&self, admin_chat_id: i64, scheduler: SchedulerHandle) {
        loop {
            let updates = self.api.get_updates();
            for update in updates {
                if !self.is_authorized(&update, admin_chat_id) {
                    log::warn!("Unauthorized access attempt from: {}", update.chat_id);
                    continue;
                }
                
                match update.kind {
                    UpdateKind::Message(msg) => self.handle_message(msg),
                    UpdateKind::CallbackQuery(query) => self.handle_callback(query, &scheduler),
                }
            }
        }
    }

    // TDD Anchor: Test authorization logic
    fn is_authorized(&self, update: &Update, admin_id: i64) -> bool {
        update.chat_id == admin_id
    }

    fn handle_message(&self, msg: Message) {
        if msg.text == "/start" {
            self.show_main_menu(msg.chat_id);
        }
    }

    fn handle_callback(&self, query: CallbackQuery, scheduler: &SchedulerHandle) {
        match query.data.as_str() {
            "status" => self.show_status(query.chat_id, scheduler),
            "maintain_now" => self.show_maintain_menu(query.chat_id),
            "maintain_core" => self.execute_maintain_core(query.chat_id),
            "maintain_rules" => self.execute_maintain_rules(query.chat_id),
            "schedule_menu" => self.show_schedule_menu(query.chat_id, scheduler),
            "schedule_core" => self.toggle_schedule(query.chat_id, scheduler, JobType::Core),
            "schedule_rules" => self.toggle_schedule(query.chat_id, scheduler, JobType::Rules),
            "view_logs" => self.show_logs(query.chat_id),
            "reboot_confirm" => self.show_reboot_confirm(query.chat_id),
            "reboot_now" => self.execute_reboot(query.chat_id),
            _ => log::warn!("Unknown callback: {}", query.data),
        }
    }
}
```

## 4. System Module (`system.rs`)

```rust
// TDD Anchor: Mock system calls for testing without actual execution
trait SystemOps {
    fn execute_script(&self, path: &str, timeout: Duration) -> Result<String, SystemError>;
    fn get_timezone(&self) -> String;
    fn check_file_exists(&self, path: &str) -> bool;
    fn reboot(&self) -> Result<(), SystemError>;
    fn get_logs(&self, service: &str, lines: usize) -> Result<String, SystemError>;
}

struct RealSystem;

impl SystemOps for RealSystem {
    fn execute_script(&self, path: &str, timeout: Duration) -> Result<String, SystemError> {
        // Use std::process::Command
        // Wait for output or timeout
        // Read result file (/tmp/vps_maintain_result.txt) if script succeeds
    }
    
    // ... implement other methods
}
```

## 5. Scheduler Module (`scheduler.rs`)

```rust
struct Scheduler {
    jobs: HashMap<JobType, CronSchedule>,
    state_path: PathBuf,
}

enum JobType {
    Core,
    Rules,
}

impl Scheduler {
    // TDD Anchor: Test adding/removing jobs updates the state file
    fn add_job(&mut self, job_type: JobType, schedule: &str) {
        self.jobs.insert(job_type, schedule.parse().unwrap());
        self.save_state();
    }
    
    fn remove_job(&mut self, job_type: JobType) {
        self.jobs.remove(&job_type);
        self.save_state();
    }
    
    fn run(&self) {
        loop {
            // Check if any job is due
            // If due, execute corresponding system script
            // Sleep for short interval
        }
    }
    
    fn save_state(&self) {
        // Serialize self.jobs to JSON and write to self.state_path
    }
    
    fn load_state(&mut self) {
        // Read JSON from self.state_path and populate self.jobs
    }
}
```

## 6. TDD Strategy

1.  **Config Tests**:
    *   Test `Config::from_env` with valid and invalid env vars.
2.  **Scheduler Tests**:
    *   Test `add_job` / `remove_job` logic.
    *   Test persistence (save/load state).
    *   Test cron parsing.
3.  **System Tests (Mocked)**:
    *   Mock `SystemOps` trait.
    *   Test `execute_script` handles success, failure, and timeout.
    *   Test `get_logs` output formatting.
4.  **Bot Logic Tests**:
    *   Test `is_authorized`.
    *   Test callback routing (ensure correct methods are called for specific data).

## 7. Implementation Steps

1.  **Setup**: Initialize Cargo project, add dependencies (`teloxide` or `reqwest`, `tokio`, `serde`, `serde_json`, `cron`, `log`, `env_logger`).
2.  **Core Logic**: Implement `SystemOps` trait and `Scheduler` logic with unit tests.
3.  **Bot Interface**: Implement Telegram interaction logic using a library like `teloxide`.
4.  **Integration**: Connect Bot commands to System/Scheduler operations.
5.  **Refinement**: Add error handling, logging, and ensure robust failure recovery.