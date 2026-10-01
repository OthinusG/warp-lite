#[cfg(target_os = "macos")]
#[path = "mac.rs"]
mod imp;

#[cfg(windows)]
#[path = "windows.rs"]
mod imp;

#[cfg(not(any(target_os = "macos", windows)))]
#[path = "noop.rs"]
mod imp;

pub use imp::Guard;

/// Returns a guard that prevents system idle-sleep while it is held.
///
/// `reason` is passed to the platform and may appear in user-visible power
/// assertions or logs.
pub fn prevent_sleep(reason: &'static str) -> Guard {
    imp::prevent_sleep(reason)
}
