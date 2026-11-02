//! Launch at login, through SMAppService (macOS 13+).

// The only unsafe code of TunnelBar: Objective-C calls of the ServiceManagement framework.
#![allow(unsafe_code)]

use objc2_service_management::{SMAppService, SMAppServiceStatus};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginItem {
    Enabled,
    Disabled,
    /// Registered, but the user must allow it in System Settings › General › Login Items.
    RequiresApproval,
    /// Not available, e.g. when running outside an application bundle.
    Unavailable,
}

pub fn status() -> LoginItem {
    // SAFETY: `mainAppService` and `status` take no arguments and have no preconditions;
    // they only read the registration state of the running app's own bundle.
    let status = unsafe { SMAppService::mainAppService().status() };
    match status {
        SMAppServiceStatus::Enabled => LoginItem::Enabled,
        SMAppServiceStatus::NotRegistered => LoginItem::Disabled,
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
