use crate::domain::errors::AppError;
use crate::domain::hardware::{BluetoothInfo, HardwareState, VolumeInfo, WifiInfo};

pub fn get_hardware_state() -> Result<HardwareState, AppError> {
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

pub fn set_system_volume(_level: u8) -> Result<(), AppError> {
    Ok(())
}

pub fn set_volume_muted(_muted: bool) -> Result<(), AppError> {
    Ok(())
}

pub fn set_wifi_enabled(_enabled: bool) -> Result<(), AppError> {
    Ok(())
}

pub fn set_bluetooth_enabled(_enabled: bool) -> Result<(), AppError> {
    Ok(())
}
