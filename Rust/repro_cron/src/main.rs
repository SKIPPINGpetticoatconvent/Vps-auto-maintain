use tokio_cron_scheduler::{Job, JobScheduler};
use std::time::Duration;
use chrono::Local;

#[tokio::main]
async fn main() {
    let mut sched = JobScheduler::new().await.unwrap();
    
    // Test 5 fields: "0 4 * * *"
    // Intent: Every day at 04:00
    // Possible Issue: Every hour at X:04:00 (due to seconds field being implied/shifted)
    let cron_5 = "0 4 * * *";
    /*
    println!("Testing cron: '{}'", cron_5);

    // We can't easily inspect the internal schedule directly via JobScheduler public API without adding a job.
    // However, JobScheduler::add returns a JobId. 
    // Wait, let's just create a Job and see if it errors or warns, or better, use `chrono` or `cron` crate directly if we could, 
    // but we want to test `tokio-cron-scheduler` behavior.
    
    // We'll create a job and see if we can get next tick.
    // Actually `Job` struct has `next_tick` method? No, it's usually internal.
    // But `JobScheduler` has `next_tick_for_job`.
    
    let job = Job::new_async(cron_5, |_uuid, _l| Box::pin(async {
        println!("Job ran");
    })).expect("Failed to create job");
    
    let job_guid = sched.add(job).await.expect("Failed to add job");
    
    // Check next tick
    if let Some(next) = sched.next_tick_for_job(job_guid).await.expect("Failed to get next tick") {
        println!("Next tick for '{}': {:?}", cron_5, next);
        // Also print it in Local time
        let next_local = next.with_timezone(&Local);
        println!("Next tick (Local): {}", next_local);
    } else {
        println!("No next tick found for '{}'", cron_5);
    }
    */

    // Also test 6 fields: "0 0 4 * * *" (sec min hour day mon weekday)
    // Intent: 04:00:00 every day
    let cron_6 = "0 0 4 * * *";
    println!("Testing cron: '{}'", cron_6);

    let job6 = Job::new_async(cron_6, |_uuid, _l| Box::pin(async {
        println!("Job ran");
    })).expect("Failed to create job 6");
    
    let job_guid6 = sched.add(job6).await.expect("Failed to add job 6");
    
    if let Some(next) = sched.next_tick_for_job(job_guid6).await.expect("Failed to get next tick") {
        println!("Next tick for '{}': {:?}", cron_6, next);
        let next_local = next.with_timezone(&Local);
        println!("Next tick (Local): {}", next_local);
    }
    
    let output_cron = if cron_5.split_whitespace().count() == 5 {
        format!("0 {}", cron_5)
    } else {
        cron_5.to_string()
    };
    println!("Adjusted cron: '{}'", output_cron);
    
    let job = Job::new_async(output_cron.as_str(), |_uuid, _l| Box::pin(async {
        println!("Job ran");
    })).expect("Failed to create job");
    
    sched.add(job).await.expect("Failed to add job");
}
