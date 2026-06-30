use crate::domain::errors::AppError;
use crate::domain::hardware::HardwareState;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use crate::domain::hardware::{BluetoothInfo, VolumeInfo, WifiInfo};

#[cfg(target_os = "windows")]
use super::windows;

#[cfg(target_os = "linux")]
use super::linux;

pub async fn get_hardware_state() -> Result<HardwareState, AppError> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            windows::get_hardware_state()
        }
        #[cfg(target_os = "linux")]
        {
            linux::get_hardware_state()
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Ok(HardwareState {
                volume: VolumeInfo {
                    level: 0,
                    muted: false,
                    available: false,
                },
                wifi: WifiInfo {
                    enabled: false,
                    available: false,
                },
                bluetooth: BluetoothInfo {
                    enabled: false,
                    available: false,
                },
            })
        }
    })
    .await
    .map_err(|e| AppError::SystemError(format!("spawn blocking: {e}")))?
}

pub async fn set_system_volume(level: u8) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            windows::set_system_volume(level)
        }
        #[cfg(target_os = "linux")]
        {
            linux::set_system_volume(level)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(AppError::NotAvailable("platform not supported".into()))
        }
    })
    .await
    .map_err(|e| AppError::SystemError(format!("spawn blocking: {e}")))?
}

pub async fn set_volume_muted(muted: bool) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            windows::set_volume_muted(muted)
        }
        #[cfg(target_os = "linux")]
        {
            linux::set_volume_muted(muted)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(AppError::NotAvailable("platform not supported".into()))
        }
    })
    .await
    .map_err(|e| AppError::SystemError(format!("spawn blocking: {e}")))?
}

pub async fn set_wifi_enabled(enabled: bool) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            windows::set_wifi_enabled(enabled)
        }
        #[cfg(target_os = "linux")]
        {
            linux::set_wifi_enabled(enabled)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(AppError::NotAvailable("platform not supported".into()))
        }
    })
    .await
    .map_err(|e| AppError::SystemError(format!("spawn blocking: {e}")))?
}

pub async fn set_bluetooth_enabled(enabled: bool) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            windows::set_bluetooth_enabled(enabled)
        }
        #[cfg(target_os = "linux")]
        {
            linux::set_bluetooth_enabled(enabled)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(AppError::NotAvailable("platform not supported".into()))
        }
    })
    .await
    .map_err(|e| AppError::SystemError(format!("spawn blocking: {e}")))?
}
