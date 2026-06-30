use crate::domain::errors::AppError;

use windows::core::GUID;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WiFi::{
    wlan_intf_opcode_radio_state, WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory,
    WlanOpenHandle, WlanQueryInterface, WlanSetInterface, WLAN_INTERFACE_INFO_LIST,
};

fn wlan_err(code: u32, op: &str) -> Result<(), AppError> {
    if code == 0 {
        Ok(())
    } else {
        Err(AppError::SystemError(format!("{op}: error code {code}")))
    }
}

fn open_wlan_client() -> Result<HANDLE, AppError> {
    unsafe {
        let mut client_handle = HANDLE::default();
        let mut negotiated_version = 0u32;
        wlan_err(
            WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle),
            "WlanOpenHandle",
        )?;
        Ok(client_handle)
    }
}

fn first_wifi_interface_guid(client_handle: HANDLE) -> Result<GUID, AppError> {
    unsafe {
        let mut interface_list_ptr = std::ptr::null_mut();
        wlan_err(
            WlanEnumInterfaces(client_handle, None, &mut interface_list_ptr),
            "WlanEnumInterfaces",
        )?;

        if interface_list_ptr.is_null() {
            return Err(AppError::NotAvailable("no Wi-Fi interface".into()));
        }

        let result = (|| -> Result<GUID, AppError> {
            let list = &*(interface_list_ptr as *const WLAN_INTERFACE_INFO_LIST);
            let count = list.dwNumberOfItems as usize;
            if count == 0 {
                return Err(AppError::NotAvailable("no Wi-Fi interface".into()));
            }
            let interfaces = std::slice::from_raw_parts(list.InterfaceInfo.as_ptr(), count);
            Ok(interfaces[0].InterfaceGuid)
        })();

        WlanFreeMemory(interface_list_ptr as *const _);
        result
    }
}

pub(crate) fn get_wifi_radio_state() -> Result<bool, AppError> {
    unsafe {
        let client_handle = open_wlan_client()?;
        let result = (|| -> Result<bool, AppError> {
            let guid = first_wifi_interface_guid(client_handle)?;
            let mut data_size: u32 = 0;
            let mut data: *mut std::ffi::c_void = std::ptr::null_mut();

            wlan_err(
                WlanQueryInterface(
                    client_handle,
                    &guid,
                    wlan_intf_opcode_radio_state,
                    None,
                    &mut data_size,
                    &mut data,
                    None,
                ),
                "WlanQueryInterface",
            )?;

            let on = *(data as *const u32) != 0;
            WlanFreeMemory(data as *const _);
            Ok(on)
        })();
        let _ = WlanCloseHandle(client_handle, None);
        result
    }
}

pub(crate) fn set_wifi_radio_state(enabled: bool) -> Result<(), AppError> {
    unsafe {
        let client_handle = open_wlan_client()?;
        let result = (|| -> Result<(), AppError> {
            let guid = first_wifi_interface_guid(client_handle)?;
            let new_state: u32 = if enabled { 1 } else { 0 };
            wlan_err(
                WlanSetInterface(
                    client_handle,
                    &guid,
                    wlan_intf_opcode_radio_state,
                    std::mem::size_of::<u32>() as u32,
                    &new_state as *const u32 as *const std::ffi::c_void,
                    None,
                ),
                "WlanSetInterface",
            )?;
            Ok(())
        })();
        let _ = WlanCloseHandle(client_handle, None);
        result
    }
}
