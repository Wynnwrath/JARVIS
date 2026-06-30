use crate::domain::errors::AppError;
use crate::domain::hardware::{BluetoothInfo, HardwareState, VolumeInfo, WifiInfo};

use windows::core::{GUID, HRESULT};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID::from_values(
    0xBCDE0395,
    0xE52F,
    0x467C,
    [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
);

fn audio_endpoint_volume() -> Result<IAudioEndpointVolume, AppError> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        match hr {
            HRESULT(0) | HRESULT(1) => {}
            h if h.0 == 0x80010106_u32 as i32 => {
                return Err(AppError::SystemError("COM threading mode conflict".into()))
            }
            _ => return Err(AppError::SystemError(format!("CoInitializeEx: {hr}"))),
        }

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&CLSID_MMDEVICE_ENUMERATOR, None, CLSCTX_ALL)
                .map_err(|e| AppError::SystemError(format!("CoCreateInstance: {e}")))?;

        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(|e| AppError::SystemError(format!("GetDefaultAudioEndpoint: {e}")))?;

        let volume: IAudioEndpointVolume = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| AppError::SystemError(format!("Activate: {e}")))?;

        Ok(volume)
    }
}

pub(crate) fn get_volume_raw() -> Result<(u8, bool), AppError> {
    let volume = audio_endpoint_volume()?;
    unsafe {
        let scalar = volume
            .GetMasterVolumeLevelScalar()
            .map_err(|e| AppError::SystemError(format!("GetMasterVolumeLevelScalar: {e}")))?;
        let level = (scalar * 100.0).round() as u8;

        let muted = volume
            .GetMute()
            .map_err(|e| AppError::SystemError(format!("GetMute: {e}")))?;

        Ok((level, muted.0 != 0))
    }
}

pub(crate) fn get_hardware_state() -> Result<HardwareState, AppError> {
    let (volume_level, volume_muted, volume_available) = match get_volume_raw() {
        Ok((level, muted)) => (level, muted, true),
        Err(_) => (0, false, false),
    };

    let (wifi_enabled, wifi_available) = match super::wlan::get_wifi_radio_state() {
        Ok(enabled) => (enabled, true),
        Err(_) => (false, false),
    };

    let (bt_enabled, bt_available) = match super::bluetooth::get_bluetooth_radio_state() {
        Ok(enabled) => (enabled, true),
        Err(_) => (false, false),
    };

    Ok(HardwareState {
        volume: VolumeInfo {
            level: volume_level,
            muted: volume_muted,
            available: volume_available,
        },
        wifi: WifiInfo {
            enabled: wifi_enabled,
            available: wifi_available,
        },
        bluetooth: BluetoothInfo {
            enabled: bt_enabled,
            available: bt_available,
        },
    })
}

pub(crate) fn set_system_volume(level: u8) -> Result<(), AppError> {
    let volume = audio_endpoint_volume()?;
    unsafe {
        volume
            .SetMasterVolumeLevelScalar(level as f32 / 100.0, std::ptr::null())
            .map_err(|e| AppError::SystemError(format!("SetMasterVolumeLevelScalar: {e}")))?;
    }
    Ok(())
}

pub(crate) fn set_volume_muted(muted: bool) -> Result<(), AppError> {
    let volume = audio_endpoint_volume()?;
    unsafe {
        volume
            .SetMute(muted, std::ptr::null())
            .map_err(|e| AppError::SystemError(format!("SetMute: {e}")))?;
    }
    Ok(())
}
