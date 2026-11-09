#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub mod generic;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub use self::generic::*;
