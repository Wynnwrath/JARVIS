#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
mod linux;

mod system;

pub use system::{
    get_hardware_state, set_bluetooth_enabled, set_system_volume, set_volume_muted,
    set_wifi_enabled,
};
