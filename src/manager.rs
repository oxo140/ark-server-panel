use crate::model::Profile;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use sysinfo::{Pid, System};

// Avoid reading every process environment, command line and memory on each status poll.
pub fn process_snapshot(pid: Option<u32>) -> System {
    let mut system = System::new();
    let pids = pid.map(|id| [Pid::from_u32(id)]);
    let selection = match &pids {
        Some(ids) => sysinfo::ProcessesToUpdate::Some(ids),
        None => sysinfo::ProcessesToUpdate::All,
    };
    system.refresh_processes_specifics(
        selection,
        true,
        sysinfo::ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
    );
    system
}

pub fn hidden(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
pub fn executable(p: &Profile) -> PathBuf {
    Path::new(&p.directory).join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe")
}
fn normalized_executable(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .to_lowercase()
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Identity {
    pub pid: u32,
    pub start: u64,
    pub executable: String,
}
pub struct Manager {
    pub identities: HashMap<String, Identity>,
    pub children: HashMap<String, Child>,
    pub jobs: HashMap<String, Child>,
    pub job_identities: HashMap<String, Identity>,
    window_guards: HashMap<String, (u32, u64)>,
    pub data: PathBuf,
}
impl Manager {
    pub fn new(data: PathBuf) -> Result<Self, String> {
        Ok(Self {
            identities: crate::model::load(&data.join("processes.json"))?,
            children: HashMap::new(),
            jobs: HashMap::new(),
            job_identities: crate::model::load(&data.join("jobs.json"))?,
            window_guards: HashMap::new(),
            data,
        })
    }
    pub fn persist(&self) -> Result<(), String> {
        crate::model::save(&self.data.join("processes.json"), &self.identities)
    }
    fn guard_windows(&mut self, id: &str, pid: u32, start: u64) {
        if self.window_guards.get(id) != Some(&(pid, start)) {
            self.window_guards.insert(id.into(), (pid, start));
            crate::window_guard::hide_server_windows(pid, start);
        }
    }
    pub fn discover(&mut self, p: &Profile) {
        let exe = executable(p);
        let system = process_snapshot(None);
        if let Some((pid, process)) = system.processes().iter().find(|(_, process)| {
            process
                .exe()
                .is_some_and(|e| normalized_executable(e) == normalized_executable(&exe))
        }) {
            self.guard_windows(&p.id, pid.as_u32(), process.start_time());
            self.identities.insert(
                p.id.clone(),
                Identity {
                    pid: pid.as_u32(),
                    start: process.start_time(),
                    executable: normalized_executable(&exe),
                },
            );
        }
    }
    pub fn running(&mut self, id: &str) -> bool {
        if let Some(child) = self.children.get_mut(id) {
            match child.try_wait() {
                Ok(None) => return true,
                Ok(Some(status)) => {
                    if let Ok(mut log) = self.log_file(id) {
                        use std::io::Write;
                        let _ = writeln!(
                            log,
                            "\n[ARK Panel {}] Processus ARK terminé : {status}. Aucun arrêt automatique n’est envoyé lorsque le serveur devient prêt. En cas d’arrêt inattendu, consulter les événements Windows et la mémoire virtuelle.",
                            crate::now()
                        );
                    }
                    self.children.remove(id);
                }
                Err(_) => return true,
            }
        }
        let Some(identity) = self.identities.get(id) else {
            return false;
        };
        let system = process_snapshot(Some(identity.pid));
        system
            .process(Pid::from_u32(identity.pid))
            .map(|p| {
                p.start_time() == identity.start
                    && p.exe()
                        .map(|p| {
                            normalized_executable(p)
                                == normalized_executable(Path::new(&identity.executable))
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }
    pub fn busy(&mut self, id: &str) -> bool {
        self.job_status(id)["running"] == true
    }
    pub fn job_status(&mut self, id: &str) -> serde_json::Value {
        let stored =
            crate::model::load::<serde_json::Value>(&self.data.join(format!("job-{id}.json")))
                .unwrap_or(serde_json::Value::Null);
        if stored["running"] == false {
            self.jobs.remove(id);
            self.job_identities.remove(id);
            return stored;
        }
        if let Some(child) = self.jobs.get_mut(id) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.jobs.remove(id);
                    let value = serde_json::json!({"running":false,"success":false,"exitCode":status.code(),"kind":stored["kind"],"message":"La console SteamCMD s’est fermée sans confirmer la fin de la tâche. Consultez le terminal."});
                    let _ = crate::model::save(&self.data.join(format!("job-{id}.json")), &value);
                    value
                }
                _ => {
                    if stored.is_object() {
                        stored
                    } else {
                        serde_json::json!({"running":true})
                    }
                }
            }
        } else {
            if stored["running"] == true {
                let alive = self.job_identities.get(id).is_some_and(|identity| {
                    let system = process_snapshot(Some(identity.pid));
                    system
                        .process(Pid::from_u32(identity.pid))
                        .is_some_and(|p| {
                            p.start_time() == identity.start
                                && p.exe().is_some_and(|e| {
                                    normalized_executable(e)
                                        == normalized_executable(Path::new(&identity.executable))
                                })
                        })
                });
                if alive {
                    return stored;
                }
                let value = serde_json::json!({"running":false,"success":false,"kind":stored["kind"],"message":"Tâche SteamCMD interrompue. Vérifiez le journal avant de relancer."});
                let _ = crate::model::save(&self.data.join(format!("job-{id}.json")), &value);
                return value;
            }
            stored
        }
    }
    pub fn start(&mut self, p: &Profile) -> Result<(), String> {
        if self.running(&p.id) || self.busy(&p.id) {
            return Err("Le serveur tourne déjà ou une opération est en cours.".into());
        }
        let exe = executable(p);
        if !exe.is_file() {
            return Err(
                "ArkAscendedServer.exe introuvable. Installer le serveur avec SteamCMD.".into(),
            );
        }
        check_launch_memory(&p.map)?;
        let log = self.log_file(&p.id)?;
        let mut cmd = Command::new(&exe);
        cmd.current_dir(exe.parent().unwrap())
            .arg(format!("{}?listen?SessionName={}", p.map, p.name))
            .arg(format!("-Port={}", p.game_port))
            .arg(format!("-RCONPort={}", p.rcon_port))
            .args([
                "-stdout",
                "-FullStdOutLogOutput",
                "-unattended",
                "-NoConsole",
            ]);
        if !p.mods.is_empty() {
            cmd.arg(format!("-mods={}", p.mods));
        }
        // No shell, detached from the browser and retained when the panel exits.
        let child = hidden(&mut cmd)
            .stdin(Stdio::null())
            .stdout(log.try_clone().map_err(|e| e.to_string())?)
            .stderr(log)
            .spawn()
            .map_err(|e| e.to_string())?;
        let pid = child.id();
        let system = process_snapshot(Some(pid));
        let start = system
            .process(Pid::from_u32(pid))
            .map(|p| p.start_time())
            .unwrap_or(0);
        self.identities.insert(
            p.id.clone(),
            Identity {
                pid,
                start,
                executable: normalized_executable(&exe),
            },
        );
        self.guard_windows(&p.id, pid, start);
        self.children.insert(p.id.clone(), child);
        self.persist()
    }
    fn log_file(&self, id: &str) -> Result<fs::File, String> {
        fs::create_dir_all(self.data.join("logs")).map_err(|e| e.to_string())?;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.data.join("logs").join(format!("{id}.log")))
            .map_err(|e| e.to_string())
    }
    pub fn install(&mut self, p: &Profile, steamcmd: &str, visible: bool) -> Result<(), String> {
        if self.running(&p.id) || self.busy(&p.id) {
            return Err(
                "Arrêter le serveur et attendre la fin des opérations avant installation.".into(),
            );
        }
        if !crate::steamcmd::valid_executable(Path::new(steamcmd)) {
            return Err("Renseigner le chemin absolu de steamcmd.exe dans les paramètres.".into());
        }
        fs::create_dir_all(&p.directory).map_err(|e| e.to_string())?;
        self.steam_job(
            &p.id,
            "install",
            steamcmd,
            Some(Path::new(&p.directory)),
            visible,
        )
    }
    pub fn steam_job(
        &mut self,
        id: &str,
        kind: &str,
        steamcmd: &str,
        directory: Option<&Path>,
        visible: bool,
    ) -> Result<(), String> {
        if id.is_empty()
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err("Identifiant de tâche SteamCMD invalide.".into());
        }
        if self.busy(id) || self.busy("_steamcmd") || (id == "_steamcmd" && self.any_busy()) {
            return Err(
                "SteamCMD travaille déjà sur ce serveur, ou son installation est en cours.".into(),
            );
        }
        fs::create_dir_all(self.data.join("logs")).map_err(|e| e.to_string())?;
        let spec = crate::console::JobSpec {
            run_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            steamcmd: PathBuf::from(steamcmd),
            directory: directory.map(Path::to_path_buf),
            log: steam_log_path(&self.data, id),
            result: self.data.join(format!("job-{id}.json")),
            visible,
            runtime: if id == "_steamcmd" {
                None
            } else {
                Some(self.data.join("steamcmd-workers").join(id))
            },
        };
        let child = crate::console::launch(&spec, &self.data.join(format!("job-spec-{id}.json")))?;
        let pid = child.id();
        let system = process_snapshot(Some(pid));
        let start = system
            .process(Pid::from_u32(pid))
            .map(|p| p.start_time())
            .unwrap_or(0);
        self.job_identities.insert(
            id.into(),
            Identity {
                pid,
                start,
                executable: normalized_executable(
                    &std::env::current_exe().map_err(|e| e.to_string())?,
                ),
            },
        );
        self.jobs.insert(id.into(), child);
        crate::model::save(&self.data.join("jobs.json"), &self.job_identities)
    }
    pub fn logs(&self, p: &Profile) -> Result<String, String> {
        let candidates = [
            steam_log_path(&self.data, &p.id),
            self.data.join("logs").join(format!("{}.log", p.id)),
            Path::new(&p.directory).join("ShooterGame/Saved/Logs/ShooterGame.log"),
        ];
        let mut result = String::new();
        for file in candidates {
            if let Ok(mut f) = fs::File::open(&file) {
                let size = f.metadata().map_err(|e| e.to_string())?.len();
                f.seek(SeekFrom::Start(size.saturating_sub(48 * 1024)))
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                f.take(48 * 1024)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                result.push_str(&format!(
                    "\n── {} ──\n{}",
                    file.file_name().unwrap().to_string_lossy(),
                    String::from_utf8_lossy(&bytes)
                ));
            }
        }
        if result.is_empty() {
            result = "Aucun journal disponible. Lancez une installation ou un serveur.".into();
        }
        Ok(result)
    }
    pub fn any_busy(&mut self) -> bool {
        let ids: std::collections::HashSet<_> = self
            .jobs
            .keys()
            .chain(self.job_identities.keys())
            .cloned()
            .collect();
        ids.iter().any(|id| self.busy(id))
    }
}
// Available page file means remaining system commit on Windows, including RAM.
// ARK can require considerably more during map loading: this is a minimum guard,
// not a promise that every map/mod combination will fit.
fn check_launch_memory(map: &str) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut memory = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..std::mem::zeroed()
        };
        if GlobalMemoryStatusEx(&mut memory) != 0 {
            validate_launch_memory(memory.ullAvailPageFile, map)?;
        }
    }
    Ok(())
}
fn validate_launch_memory(available: u64, map: &str) -> Result<(), String> {
    // Lost Colony exceeded 13 GiB of commit in the reported startup.
    let minimum_gib = if map.eq_ignore_ascii_case("LostColony_WP") {
        16
    } else {
        8
    };
    if available < minimum_gib * 1024 * 1024 * 1024 {
        return Err(format!(
            "Mémoire virtuelle disponible insuffisante ({:.1} Gio). Au moins {minimum_gib} Gio de marge sont requis avant de lancer cette carte ; certaines cartes et mods demandent davantage. Fermez des applications ou activez/augmentez le fichier d’échange Windows pour éviter la fermeture des serveurs et du panneau.",
            available as f64 / (1024. * 1024. * 1024.)
        ));
    }
    Ok(())
}
#[cfg(test)]
mod memory_tests {
    #[test]
    fn lost_colony_reserves_its_observed_loading_peak() {
        assert!(super::validate_launch_memory(14 * 1024 * 1024 * 1024, "LostColony_WP").is_err());
        assert!(super::validate_launch_memory(16 * 1024 * 1024 * 1024, "LostColony_WP").is_ok());
    }
    #[test]
    fn insufficient_commit_prevents_another_server_launch() {
        assert!(super::validate_launch_memory(1024 * 1024 * 1024, "TheIsland_WP").is_err());
        assert!(super::validate_launch_memory(8 * 1024 * 1024 * 1024, "TheIsland_WP").is_ok());
    }
}
pub fn steam_log_path(data: &Path, id: &str) -> PathBuf {
    data.join("logs").join(if id == "_steamcmd" {
        "steamcmd.log".into()
    } else {
        format!("{id}-steamcmd.log")
    })
}
pub fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(from).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Lien ou jonction refusé dans une sauvegarde.".into());
        }
    }
    if metadata.file_type().is_symlink() {
        return Err("Lien symbolique refusé dans une sauvegarde.".into());
    }
    if metadata.is_dir() {
        fs::create_dir_all(to).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        fs::copy(from, to).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn hash_tree(dir: &Path) -> Result<Vec<(String, String)>, String> {
    fn visit(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("Jonction refusée.".into());
                }
            }
            if meta.file_type().is_symlink() {
                return Err("Lien refusé.".into());
            }
            if meta.is_dir() {
                visit(root, &entry.path(), out)?;
            } else {
                let mut file = fs::File::open(entry.path()).map_err(|e| e.to_string())?;
                let mut hash = Sha256::new();
                let mut buffer = [0; 65536];
                loop {
                    let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    hash.update(&buffer[..n]);
                }
                out.push((
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into(),
                    format!("{:x}", hash.finalize()),
                ));
            }
        }
        Ok(())
    }
    let mut result = vec![];
    visit(dir, dir, &mut result)?;
    result.sort();
    Ok(result)
}
