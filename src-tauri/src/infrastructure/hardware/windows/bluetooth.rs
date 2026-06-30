use crate::domain::errors::AppError;

use std::os::windows::process::CommandExt;

pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Validate a PnP InstanceId contains only safe characters (alphanumeric, `-`, `\`, `&`, `_`).
/// Guards against PowerShell command injection via the `-InstanceId` argument.
pub fn is_valid_instance_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'\\' | b'&' | b'_'))
}

fn find_bluetooth_radio_instance_id() -> Result<String, AppError> {
    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-Command",
            "Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | \
             Where-Object { $_.InstanceId -match '^(USB|PCI)\\\\' } | \
             Select-Object -First 1 -ExpandProperty InstanceId",
        ])
        .output()
        .map_err(|e| AppError::SystemError(format!("powershell: {e}")))?;

    if !output.status.success() {
        return Err(AppError::SystemError(format!(
            "Get-PnpDevice failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let instance_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if instance_id.is_empty() {
        Err(AppError::NotAvailable("no Bluetooth radio".into()))
    } else {
        Ok(instance_id)
    }
}

pub(crate) fn get_bluetooth_radio_state() -> Result<bool, AppError> {
    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-Command",
            "Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | \
             Where-Object { $_.InstanceId -match '^(USB|PCI)\\\\' } | \
             Select-Object -First 1 -ExpandProperty Status",
        ])
        .output()
        .map_err(|e| AppError::SystemError(format!("powershell: {e}")))?;

    if !output.status.success() {
        return Err(AppError::SystemError(format!(
            "Get-PnpDevice failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if status.is_empty() {
        Err(AppError::NotAvailable("no Bluetooth radio".into()))
    } else {
        Ok(status == "OK")
    }
}

pub(crate) fn set_bluetooth_radio_state(enabled: bool) -> Result<(), AppError> {
    let instance_id = find_bluetooth_radio_instance_id()?;
    if !is_valid_instance_id(&instance_id) {
        return Err(AppError::SystemError(
            "InstanceId contains invalid characters".into(),
        ));
    }
    let verb = if enabled { "Enable" } else { "Disable" };

    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-Command",
            &format!("param([string]$Id) {verb}-PnpDevice -InstanceId $Id -Confirm:$false"),
            &instance_id,
        ])
        .output()
        .map_err(|e| AppError::SystemError(format!("powershell: {e}")))?;

    if !output.status.success() {
        return Err(AppError::SystemError(format!(
            "{verb}-PnpDevice failed (likely requires admin): {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}
