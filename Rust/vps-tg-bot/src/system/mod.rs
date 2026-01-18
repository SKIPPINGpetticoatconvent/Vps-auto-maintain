#[cfg(test)]
pub mod error_tests;
pub mod errors;
pub mod info;
pub mod ops;
pub mod update;

#[allow(unused_imports)]
pub use errors::SystemError;
pub use info::get_system_status;
#[allow(unused_imports)]
pub use info::SystemStatus;
#[allow(unused_imports)]
pub use ops::{
    check_security_updates, perform_full_maintenance, perform_maintenance, reboot_system,
    restart_service,
};
