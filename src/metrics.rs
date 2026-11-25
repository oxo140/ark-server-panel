use serde_json::{Value, json};
use std::{
    path::PathBuf,
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
fn retain_recent(samples: &mut Vec<Value>, now: u64) {
    samples.retain(|v| {
        v["at"]
            .as_u64()
            .is_some_and(|at| at <= now && now.saturating_sub(at) <= 600)
    });
    samples.sort_by_key(|s| s["at"].as_u64().unwrap_or(0));
    samples.dedup_by_key(|s| s["at"].as_u64().unwrap_or(0));
    if samples.len() > 301 {
        samples.drain(..samples.len() - 301);
    }
}
pub fn start(data: PathBuf) -> Metrics {
    let state = empty();
    let output = state.clone();
    std::thread::spawn(move || {
        let mut system = System::new();
        system.refresh_cpu_all();
        system.refresh_memory();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut previous = Instant::now();
        let file = data.join("metrics-history.json");
        let mut history: Vec<Value> = crate::model::load(&file).unwrap_or_default();
        retain_recent(&mut history, crate::now());
        let mut persisted = Instant::now();
        let mut persistence_error: Option<String> = None;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let seconds = previous.elapsed().as_secs_f64();
            previous = Instant::now();
            system.refresh_cpu_usage();
            system.refresh_memory();
            system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                sysinfo::ProcessRefreshKind::nothing(),
            );
            let ark_pids: Vec<_> = system
                .processes()
                .iter()
                .filter(|(_, p)| p.name().eq_ignore_ascii_case("ArkAscendedServer.exe"))
                .map(|(pid, _)| *pid)
                .collect();
            system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::Some(&ark_pids),
                false,
                sysinfo::ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory()
                    .with_disk_usage(),
            );
            networks.refresh(true);
            disks.refresh(true);
            let interfaces:Vec<_>=networks.iter().map(|(name,data)|json!({"name":name,"receivedBytesPerSec":rate(data.received(),seconds),"transmittedBytesPerSec":rate(data.transmitted(),seconds)})).collect();
            let disk_rows:Vec<_>=disks.iter().map(|disk|{let usage=disk.usage();json!({"name":disk.name().to_string_lossy(),"mount":disk.mount_point().to_string_lossy(),"totalBytes":disk.total_space(),"availableBytes":disk.available_space(),"readBytesPerSec":rate(usage.read_bytes,seconds),"writtenBytesPerSec":rate(usage.written_bytes,seconds)})}).collect();
            let at = crate::now();
            let sum = |rows: &Vec<Value>, key: &str| {
                rows.iter()
                    .map(|v| v[key].as_f64().unwrap_or(0.))
                    .sum::<f64>()
            };
            history.push(json!({"at":at,"cpuPercent":system.global_cpu_usage(),"memoryUsedBytes":system.used_memory(),"memoryTotalBytes":system.total_memory(),"diskReadBytesPerSec":sum(&disk_rows,"readBytesPerSec"),"diskWrittenBytesPerSec":sum(&disk_rows,"writtenBytesPerSec"),"receivedBytesPerSec":sum(&interfaces,"receivedBytesPerSec"),"transmittedBytesPerSec":sum(&interfaces,"transmittedBytesPerSec")}));
            retain_recent(&mut history, at);
            if persisted.elapsed() >= Duration::from_secs(10) {
                persistence_error = crate::model::save(&file, &history).err();
                persisted = Instant::now();
            }
            let processes:serde_json::Map<String,Value>=system.processes().iter().filter(|(_,p)|p.name().eq_ignore_ascii_case("ArkAscendedServer.exe")).map(|(pid,p)|{let io=p.disk_usage();(pid.as_u32().to_string(),json!({"pid":pid.as_u32(),"startTime":p.start_time(),"sampledAt":at,"cpuPercent":p.cpu_usage()/system.cpus().len().max(1) as f32,"memoryBytes":p.memory(),"ioReadBytesPerSec":rate(io.read_bytes,seconds),"ioWrittenBytesPerSec":rate(io.written_bytes,seconds),"networkAvailable":false}))}).collect();
            let value = json!({"ready":true,"sampledAt":at,"intervalSeconds":seconds,"host":System::host_name(),"cpuPercent":system.global_cpu_usage(),"cpuName":system.cpus().first().map(|c|c.brand()),"logicalCpus":system.cpus().len(),"memory":{"usedBytes":system.used_memory(),"totalBytes":system.total_memory()},"disks":disk_rows,"interfaces":interfaces,"processes":processes,"history":history,"historyWindowSeconds":600,"historyPersistenceError":persistence_error});
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
    #[test]
    fn history_is_limited_to_ten_minutes_and_ignores_future_points() {
        let mut rows = vec![
            json!({"at":1}),
            json!({"at":500}),
            json!({"at":500}),
            json!({"at":1001}),
        ];
        retain_recent(&mut rows, 1000);
        assert_eq!(rows, vec![json!({"at":500})]);
        let mut rows = (0..1000).map(|at| json!({"at":at})).collect();
        retain_recent(&mut rows, 999);
        assert_eq!(rows.len(), 301);
        assert_eq!(rows.last().unwrap()["at"], 999);
    }
}
