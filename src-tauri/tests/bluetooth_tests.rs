#[cfg(target_os = "windows")]
mod tests {
    use jarvis_lib::infrastructure::hardware::windows::bluetooth::is_valid_instance_id;

    #[test]
    fn valid_instance_id_with_backslashes() {
        assert!(is_valid_instance_id("USB\\VID_0BDA&PID_2553\\5&12345"));
    }

    #[test]
    fn rejects_injection_attempt() {
        assert!(!is_valid_instance_id("'; Write-Host pwned ;'"));
    }

    #[test]
    fn rejects_empty_string() {
        assert!(!is_valid_instance_id(""));
    }
}
