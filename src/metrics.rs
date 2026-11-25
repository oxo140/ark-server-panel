use serde_json::{Value, json};
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use sysinfo::{Disks, Networks, System};
pub type Metrics = Arc<RwLock<Value>>;
pub fn empty() -> Metrics {
    Arc::new(RwLock::new(json!({"ready":false})))
}
pub fn rate(bytes: u64, seconds: f64) -> f64 {
    if seconds.is_finite() && seconds > 0. {
        bytes as f64 / seconds
    } else {
        0.
    }
}
pub fn start() -> Metrics {
    let state = empty();
    let output = state.clone();
    std::thread::spawn(move || {
        let mut system = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut previous = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let seconds = previous.elapsed().as_secs_f64();
            previous = Instant::now();
            system.refresh_cpu_usage();
            system.refresh_memory();
            networks.refresh(true);
            disks.refresh(true);
            let interfaces:Vec<_>=networks.iter().map(|(name,data)|json!({"name":name,"receivedBytesPerSec":rate(data.received(),seconds),"transmittedBytesPerSec":rate(data.transmitted(),seconds)})).collect();
            let disk_rows:Vec<_>=disks.iter().map(|disk|{let usage=disk.usage();json!({"name":disk.name().to_string_lossy(),"mount":disk.mount_point().to_string_lossy(),"totalBytes":disk.total_space(),"availableBytes":disk.available_space(),"readBytesPerSec":rate(usage.read_bytes,seconds),"writtenBytesPerSec":rate(usage.written_bytes,seconds)})}).collect();
            let value = json!({"ready":true,"sampledAt":crate::now(),"intervalSeconds":seconds,"host":System::host_name(),"cpuPercent":system.global_cpu_usage(),"cpuName":system.cpus().first().map(|c|c.brand()),"logicalCpus":system.cpus().len(),"memory":{"usedBytes":system.used_memory(),"totalBytes":system.total_memory()},"disks":disk_rows,"interfaces":interfaces});
            if let Ok(mut target) = output.write() {
                *target = value;
            }
        }
    });
    state
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_and_disk_rates_use_the_actual_elapsed_interval() {
        assert_eq!(rate(1200, 2.), 600.);
        assert_eq!(rate(1200, 0.), 0.);
        assert_eq!(rate(1200, f64::NAN), 0.);
    }
}
