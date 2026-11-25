// Some ASA builds create a GUI log window even when their process has no console.
// A process handle pins its identity: a reused PID can never target a different app.
#[cfg(windows)]
pub fn hide_server_windows(pid: u32, expected_start: u64) {
    use windows_sys::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
    unsafe extern "system" fn hide(hwnd: HWND, context: LPARAM) -> i32 {
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut owner);
            if owner == context as u32 && IsWindowVisible(hwnd) != 0 {
                ShowWindowAsync(hwnd, SW_HIDE);
            }
        }
        1
    }
    std::thread::spawn(move || unsafe {
        let process = OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );
        if process.is_null() {
            return;
        }
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let valid = GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) != 0;
        let start = ((u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime))
            / 10_000_000;
        if !valid || start.saturating_sub(11_644_473_600) != expected_start {
            CloseHandle(process);
            return;
        }
        let began = std::time::Instant::now();
        while WaitForSingleObject(process, 0) == WAIT_TIMEOUT {
            EnumWindows(Some(hide), pid as LPARAM);
            std::thread::sleep(std::time::Duration::from_millis(
                if began.elapsed().as_secs() < 60 {
                    250
                } else {
                    2000
                },
            ));
        }
        CloseHandle(process);
    });
}
#[cfg(not(windows))]
pub fn hide_server_windows(_pid: u32, _expected_start: u64) {}
