use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SchedulerState {
    cron_expression: String,
}

impl SchedulerState {
    fn new(cron_expression: &str) -> Self {
        Self {
            cron_expression: cron_expression.to_string(),
        }
    }

    fn default() -> Self {
        Self {
            cron_expression: "0 0 4 * * Sun".to_string(),
        }
    }

    fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json)
    }

    fn load_from_file(path: &str) -> Self {
        if !Path::new(path).exists() {
            return SchedulerState::default();
        }
        let content = fs::read_to_string(path).unwrap();
        let state: SchedulerState = serde_json::from_str(&content).unwrap();
        state
    }
}

#[test]
fn test_scheduler_state_persistence() {
    // Test creating and saving scheduler state
    let state = SchedulerState::new("0 0 * * *");
    state.save_to_file("test_scheduler_state.json").unwrap();
    
    // Test loading scheduler state
    let loaded_state = SchedulerState::load_from_file("test_scheduler_state.json");
    
    assert_eq!(state.cron_expression, loaded_state.cron_expression);
    
    // Test default state when file doesn't exist
    fs::remove_file("test_scheduler_state.json").ok();
    let default_state = SchedulerState::load_from_file("nonexistent_file.json");
    assert_eq!(default_state.cron_expression, "0 0 4 * * Sun");
    
    // Clean up
    fs::remove_file("test_scheduler_state.json").ok();
}

#[test]
fn test_cron_validation() {
    let valid_cron = "0 0 * * *";
    let invalid_cron = "0 0 * *";
    
    assert_eq!(valid_cron.split_whitespace().count(), 5);
    assert_ne!(invalid_cron.split_whitespace().count(), 5);
}