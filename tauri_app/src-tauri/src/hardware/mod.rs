pub mod mouse_engine;
pub mod remote_dispatcher;
pub mod serial_transport;
pub mod state;
pub mod watchdog;

pub use mouse_engine::spawn_mouse_revert_watchdog;
pub use serial_transport::spawn_serial_reader_thread;
pub use state::{safe_lock, AppState, PendingHudState};
pub use watchdog::spawn_usb_watchdog;
