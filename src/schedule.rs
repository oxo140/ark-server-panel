use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Schedule {
    pub backup_enabled: bool,
    pub backup_hours: u64,
    pub restart_enabled: bool,
    pub restart_time: String,
    pub update_enabled: bool,
    pub update_time: String,
    pub warning_minutes: u64,
    pub message: String,
    pub last_backup: u64,
    pub last_restart_day: u32,
    pub last_update_day: u32,
    pub last_result: String,
}
impl Default for Schedule {
    fn default() -> Self {
        Self {
            backup_enabled: false,
            backup_hours: 2,
            restart_enabled: false,
            restart_time: "06:00".into(),
            update_enabled: false,
            update_time: "05:00".into(),
            warning_minutes: 5,
            message:
                "Maintenance du serveur dans {minutes} minute(s). Reconnexion après redémarrage."
                    .into(),
            last_backup: 0,
            last_restart_day: 0,
            last_update_day: 0,
            last_result: String::new(),
        }
    }
}
pub fn minute(time: &str) -> Option<u32> {
    let (h, m) = time.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}
impl Schedule {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=168).contains(&self.backup_hours)
            || self.warning_minutes > 60
            || minute(&self.restart_time).is_none()
            || minute(&self.update_time).is_none()
            || self.message.len() > 500
            || self.message.chars().any(|c| c.is_control())
        {
            return Err("Planification invalide : intervalle 1–168 h, préavis 0–60 min, horaires HH:MM et message sans retour à la ligne (500 caractères).".into());
        }
        Ok(())
    }
    pub fn due(&self, now: u64, day: u32, current: u32) -> (bool, bool, bool) {
        (
            self.backup_enabled && now.saturating_sub(self.last_backup) >= self.backup_hours * 3600,
            self.restart_enabled
                && self.last_restart_day != day
                && minute(&self.restart_time).is_some_and(|m| current >= m),
            self.update_enabled
                && self.last_update_day != day
                && minute(&self.update_time).is_some_and(|m| current >= m),
        )
    }
}
// Windows local civil time includes the host's daylight-saving rules.
#[cfg(windows)]
pub fn local_clock() -> (u32, u32) {
    #[repr(C)]
    #[derive(Default)]
    struct Clock {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(clock: *mut Clock);
    }
    let mut c = Clock::default();
    unsafe {
        GetLocalTime(&mut c);
    }
    (
        u32::from(c.year) * 10000 + u32::from(c.month) * 100 + u32::from(c.day),
        u32::from(c.hour) * 60 + u32::from(c.minute),
    )
}
#[cfg(not(windows))]
pub fn local_clock() -> (u32, u32) {
    let n = crate::now();
    ((n / 86400) as u32, (n % 86400 / 60) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schedule_validates_and_runs_once_per_day() {
        let mut s = Schedule {
            backup_enabled: true,
            restart_enabled: true,
            update_enabled: true,
            last_backup: 100,
            ..Default::default()
        };
        assert_eq!(s.due(7299, 20261125, 360), (false, true, true));
        assert_eq!(s.due(7300, 20261125, 360), (true, true, true));
        s.last_restart_day = 20261125;
        s.last_update_day = 20261125;
        assert_eq!(s.due(7300, 20261125, 360), (true, false, false));
        s.backup_hours = 0;
        assert!(s.validate().is_err());
        assert_eq!(minute("24:00"), None);
        assert_eq!(minute("05:30"), Some(330));
    }
}
