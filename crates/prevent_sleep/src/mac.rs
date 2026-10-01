use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, NSString};

/// A guard object that prevents system sleep while it remains in scope.
pub struct Guard {
    process_info: Retained<NSProcessInfo>,
    activity_token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
    reason: Retained<NSString>,
}

// Safe to share across threads: the activity is only touched at creation and
// drop, and NSProcessInfo is itself thread-safe.
unsafe impl Send for Guard {}
unsafe impl Sync for Guard {}

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            self.process_info.endActivity(&self.activity_token);
        }
        log::info!("No longer preventing sleep with reason: {}", self.reason);
    }
}

/// Returns a guard that prevents system sleep while it remains in scope.
pub fn prevent_sleep(reason: &'static str) -> Guard {
    let reason = NSString::from_str(reason);

    let process_info = NSProcessInfo::processInfo();
    let activity_token =
        process_info.beginActivityWithOptions_reason(NSActivityOptions::UserInitiated, &reason);

    log::info!("Preventing sleep with reason: {reason}");

    Guard {
        process_info,
        activity_token,
        reason,
    }
}
