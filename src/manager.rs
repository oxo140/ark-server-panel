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
    pub data: PathBuf,
}
impl Manager {
    pub fn new(data: PathBuf) -> Result<Self, String> {
        Ok(Self {
            identities: crate::model::load(&data.join("processes.json"))?,
            children: HashMap::new(),
            jobs: HashMap::new(),
            data,
        })
    }
    pub fn persist(&self) -> Result<(), String> {
        crate::model::save(&self.data.join("processes.json"), &self.identities)
    }
    pub fn discover(&mut self, p: &Profile) {
        let exe = executable(p);
        let system = System::new_all();
        if let Some((pid, process)) = system.processes().iter().find(|(_, process)| {
            process.exe().is_some_and(|e| {
                e.to_string_lossy()
                    .eq_ignore_ascii_case(&exe.to_string_lossy())
            })
        }) {
            self.identities.insert(
                p.id.clone(),
                Identity {
                    pid: pid.as_u32(),
                    start: process.start_time(),
                    executable: exe.to_string_lossy().into(),
                },
            );
        }
    }
    pub fn running(&mut self, id: &str) -> bool {
        if let Some(child) = self.children.get_mut(id) {
            match child.try_wait() {
                Ok(None) => return true,
                Ok(Some(_)) => {
                    self.children.remove(id);
                }
                Err(_) => return true,
            }
        }
        let Some(identity) = self.identities.get(id) else {
            return false;
        };
        let system = System::new_all();
        system
            .process(Pid::from_u32(identity.pid))
            .map(|p| {
                p.start_time() == identity.start
                    && p.exe()
                        .map(|p| {
                            p.to_string_lossy()
                                .eq_ignore_ascii_case(&identity.executable)
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }
    pub fn busy(&mut self, id: &str) -> bool {
        self.job_status(id)["running"] == true
    }
    pub fn job_status(&mut self, id: &str) -> serde_json::Value {
        if let Some(child) = self.jobs.get_mut(id) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.jobs.remove(id);
                    let value = serde_json::json!({"running":false,"success":status.success(),"exitCode":status.code()});
                    let _ = crate::model::save(&self.data.join(format!("job-{id}.json")), &value);
                    value
                }
                _ => serde_json::json!({"running":true}),
            }
        } else {
            crate::model::load::<serde_json::Value>(&self.data.join(format!("job-{id}.json")))
                .unwrap_or(serde_json::Value::Null)
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
        let log = self.log_file(&p.id)?;
        let mut cmd = Command::new(&exe);
        cmd.current_dir(exe.parent().unwrap())
            .arg(format!("{}?listen?SessionName={}", p.map, p.name))
            .arg(format!("-Port={}", p.game_port))
            .arg(format!("-RCONPort={}", p.rcon_port))
            .arg("-log");
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
        let system = System::new_all();
        let start = system
            .process(Pid::from_u32(pid))
            .map(|p| p.start_time())
            .unwrap_or(0);
        self.identities.insert(
            p.id.clone(),
            Identity {
                pid,
                start,
                executable: exe.to_string_lossy().into(),
            },
        );
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
    pub fn install(&mut self, p: &Profile, steamcmd: &str) -> Result<(), String> {
        if self.running(&p.id) || self.busy(&p.id) {
            return Err(
                "Arrêter le serveur et attendre la fin des opérations avant installation.".into(),
            );
        }
        if !Path::new(steamcmd).is_absolute()
            || !steamcmd.to_ascii_lowercase().ends_with("steamcmd.exe")
            || !Path::new(steamcmd).is_file()
        {
            return Err("Renseigner le chemin absolu de steamcmd.exe dans les paramètres.".into());
        }
        fs::create_dir_all(&p.directory).map_err(|e| e.to_string())?;
        let log = self.log_file(&p.id)?;
        let child = hidden(
            Command::new(steamcmd)
                .current_dir(Path::new(steamcmd).parent().unwrap())
                .args([
                    "+force_install_dir",
                    &p.directory,
                    "+login",
                    "anonymous",
                    "+app_update",
                    "2430930",
                    "validate",
                    "+quit",
                ]),
        )
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .spawn()
        .map_err(|e| e.to_string())?;
        self.jobs.insert(p.id.clone(), child);
        Ok(())
    }
    pub fn logs(&self, p: &Profile) -> Result<String, String> {
        let candidates = [
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
        let ids: Vec<_> = self.jobs.keys().cloned().collect();
        ids.iter().any(|id| self.busy(id))
    }
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
