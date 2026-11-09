//! macOS launch at login, through SMAppService (macOS 13+).

// The only unsafe code of TunnelBar: Objective-C calls of the ServiceManagement framework.
#![allow(unsafe_code)]

use objc2_service_management::{SMAppService, SMAppServiceStatus};

use super::LoginItem;

pub fn status() -> LoginItem {
    // SAFETY: `mainAppService` and `status` take no arguments and have no preconditions;
    // they only read the registration state of the running app's own bundle.
    let status = unsafe { SMAppService::mainAppService().status() };
    from_status(status)
}

fn from_status(status: SMAppServiceStatus) -> LoginItem {
    match status {
        SMAppServiceStatus::Enabled => LoginItem::Enabled,
        // macOS 13 and 14 report NotFound for an app that never registered: registering
        // is what makes it known, so it is simply off.
        SMAppServiceStatus::NotRegistered | SMAppServiceStatus::NotFound => LoginItem::Disabled,
        SMAppServiceStatus::RequiresApproval => LoginItem::RequiresApproval,
        _ => LoginItem::Unavailable,
    }
}

pub fn set(enabled: bool) -> Result<LoginItem, String> {
    // SAFETY: same as above; (un)registering only affects the app's own login item, and
    // errors are returned as NSError values instead of being raised.
    let service = unsafe { SMAppService::mainAppService() };
    let result = if enabled {
        unsafe { service.registerAndReturnError() }
    } else {
        unsafe { service.unregisterAndReturnError() }
    };
    result.map_err(|error| error.localizedDescription().to_string())?;
    Ok(status())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_registration_states() {
        assert_eq!(from_status(SMAppServiceStatus::Enabled), LoginItem::Enabled);
        assert_eq!(
            from_status(SMAppServiceStatus::NotRegistered),
            LoginItem::Disabled
        );
        assert_eq!(
            from_status(SMAppServiceStatus::RequiresApproval),
            LoginItem::RequiresApproval
        );
    }

    #[test]
    fn an_app_never_registered_is_disabled_not_unavailable() {
        // macOS 13 and 14 report NotFound until the first registration.
        assert_eq!(
            from_status(SMAppServiceStatus::NotFound),
            LoginItem::Disabled
        );
    }

    #[test]
    fn an_unknown_state_is_unavailable() {
        assert_eq!(from_status(SMAppServiceStatus(42)), LoginItem::Unavailable);
    }
}
