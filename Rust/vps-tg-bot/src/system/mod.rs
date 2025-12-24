pub mod info;
pub mod ops;

pub use info::get_system_status;
pub use info::SystemStatus;
pub use ops::{check_security_updates, perform_maintenance, reboot_system, restart_service};