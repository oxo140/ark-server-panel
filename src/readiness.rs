use crate::{console, model::Profile};
use std::path::Path;

// Read only the current run: old successful boots must not validate a new boot.
pub fn advertised(p: &Profile, started: u64) -> bool {
    let path = Path::new(&p.directory).join("ShooterGame/Saved/Logs/ShooterGame.log");
    let modified = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    if !modified.is_some_and(|at| at >= started) {
        return false;
    }
    let text = console::tail(&path, 128 * 1024);
    // The marker must be from a log opened after this process began.
    // File mtime alone is insufficient for an old log appended to during boot.
    let marker = "Server has completed startup and is now advertising for join.";
    current_advertisement(&text, started, marker)
}
fn current_advertisement(text: &str, started: u64, marker: &str) -> bool {
    text.lines()
        .any(|line| line.contains(marker) && log_timestamp(line).is_some_and(|at| at >= started))
}
fn log_timestamp(line: &str) -> Option<u64> {
    let s = line.strip_prefix('[')?.get(..19)?;
    let v: Vec<u64> = s
        .split(['.', '-', ':'])
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if v.len() != 6 {
        return None;
    }
    let (year, month, day) = (v[0] as i64, v[1] as i64, v[2] as i64);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || v[3] > 23 || v[4] > 59 || v[5] > 59
    {
        return None;
    }
    // Gregorian civil date to Unix days; ARK log timestamps are UTC.
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    u64::try_from(days * 86400 + v[3] as i64 * 3600 + v[4] as i64 * 60 + v[5] as i64).ok()
}
#[cfg(windows)]
pub fn game_port_bound(pid: u32, port: u16) -> bool {
    use windows_sys::Win32::NetworkManagement::IpHelper::*;
    unsafe {
        let mut size = 0;
        GetExtendedUdpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            2,
            UDP_TABLE_OWNER_PID,
            0,
        );
        for _ in 0..3 {
            if !(4..=16 * 1024 * 1024).contains(&size) {
                return false;
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let result = GetExtendedUdpTable(
                buffer.as_mut_ptr().cast(),
                &mut size,
                0,
                2,
                UDP_TABLE_OWNER_PID,
                0,
            );
            if result == 122 {
                continue;
            }
            if result != 0 {
                return false;
            }
            let count = *(buffer.as_ptr().cast::<u32>()) as usize;
            let offset = std::mem::offset_of!(MIB_UDPTABLE_OWNER_PID, table);
            if offset + count.saturating_mul(std::mem::size_of::<MIB_UDPROW_OWNER_PID>())
                > buffer.len() * 8
            {
                return false;
            }
            let rows = std::slice::from_raw_parts(
                buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<MIB_UDPROW_OWNER_PID>(),
                count,
            );
            return rows
                .iter()
                .any(|r| r.dwOwningPid == pid && u16::from_be(r.dwLocalPort as u16) == port);
        }
        false
    }
}
#[cfg(not(windows))]
pub fn game_port_bound(_pid: u32, _port: u16) -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_advertisement_cannot_validate_a_new_boot() {
        let marker = "Server has completed startup and is now advertising for join.";
        let old = format!("[2026.11.25-13.46.05:761][230] {marker}");
        assert!(!super::current_advertisement(&old, 1795614400, marker));
        assert!(super::current_advertisement(&old, 1795614000, marker));
        assert!(!super::current_advertisement(
            "Server has successfully started!",
            0,
            marker
        ));
    }
    #[test]
    fn ark_log_utc_timestamp_is_parsed() {
        assert_eq!(
            super::log_timestamp("[1970.01.01-00.00.00:123][0] text"),
            Some(0)
        );
        assert_eq!(
            super::log_timestamp("[2026.11.25-13.46.05:761][230] text"),
            Some(1795614365)
        );
        assert_eq!(super::log_timestamp("old marker without timestamp"), None);
    }
    #[cfg(windows)]
    #[test]
    fn udp_port_must_belong_to_the_checked_process() {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        assert!(super::game_port_bound(std::process::id(), port));
        assert!(!super::game_port_bound(0, port));
        drop(socket);
        assert!(!super::game_port_bound(std::process::id(), port));
    }
}
