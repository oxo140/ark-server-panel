use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub directory: String,
    pub map: String,
    pub game_port: u16,
    pub rcon_port: u16,
    #[serde(default)]
    pub mods: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub profiles: Vec<Profile>,
    pub repository: String,
    pub branch: String,
    pub steamcmd: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profiles: vec![],
            repository: "oxo140/ark-server-panel".into(),
            branch: "main".into(),
            steamcmd: String::new(),
        }
    }
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("ARK_PANEL_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("LOCALAPPDATA")
                    .or_else(|| std::env::var_os("HOME"))
                    .expect("Dossier utilisateur introuvable"),
            )
            .join("ArkServerPanel")
        })
}
pub fn load<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| format!("Fichier de données invalide : {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    save_bytes(
        path,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
}
pub fn save_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Dossier manquant")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    // MoveFileEx provides replace-existing semantics on Windows, unlike remove-then-rename.
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        unsafe extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    fs::rename(&temp, path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn validate_repo(repo: &str, branch: &str) -> Result<(), String> {
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|s| {
            s.is_empty()
                || s.contains("..")
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        })
    {
        return Err("Dépôt attendu : propriétaire/nom, sans URL ni jeton.".into());
    }
    if branch != "main" {
        return Err("La V1 suit uniquement la branche main.".into());
    }
    Ok(())
}
fn normalized(p: &Path) -> PathBuf {
    PathBuf::from(
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase(),
    )
}
pub fn validate_profile(p: &Profile, profiles: &[Profile], data: &Path) -> Result<(), String> {
    if p.name.trim().is_empty() || p.name.len() > 100 {
        return Err("Nom requis, limité à 100 caractères.".into());
    }
    if p.name.chars().any(|c| c.is_control() || "?\"".contains(c)) {
        return Err(
            "Le nom ne peut pas contenir de guillemets, ? ou caractères de contrôle.".into(),
        );
    }
    let dir = Path::new(&p.directory);
    if !dir.is_absolute()
        || dir.parent().is_none()
        || dir
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Choisir un dossier absolu dédié, sans ..".into());
    }
    // Windows junctions and symlinks can alias another server or the application data.
    let mut ancestor = Some(dir);
    while let Some(a) = ancestor {
        if let Ok(m) = fs::symlink_metadata(a) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err(
                        "Les jonctions et liens ne sont pas acceptés pour un serveur.".into(),
                    );
                }
            }
            if m.file_type().is_symlink() {
                return Err("Les liens symboliques ne sont pas acceptés.".into());
            }
        }
        ancestor = a.parent();
    }
    let dir = normalized(dir);
    let data = normalized(data);
    if dir.starts_with(&data) || data.starts_with(&dir) {
        return Err("Le dossier du serveur doit être distinct des données du panneau.".into());
    }
    if p.map.is_empty() || !p.map.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Nom de carte invalide.".into());
    }
    if p.game_port < 1024 || p.rcon_port < 1024 || p.game_port == p.rcon_port {
        return Err("Deux ports distincts entre 1024 et 65535 sont requis.".into());
    }
    if !p.mods.is_empty()
        && p.mods
            .split(',')
            .any(|s| s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()))
    {
        return Err("Mods : identifiants numériques séparés par des virgules.".into());
    }
    for other in profiles.iter().filter(|o| o.id != p.id) {
        let other_dir = normalized(Path::new(&other.directory));
        if dir.starts_with(&other_dir) || other_dir.starts_with(&dir) {
            return Err("Ce dossier recouvre celui d’un autre profil.".into());
        }
        if [other.game_port, other.rcon_port]
            .iter()
            .any(|n| *n == p.game_port || *n == p.rcon_port)
        {
            return Err("Un port est déjà réservé par un autre profil.".into());
        }
    }
    Ok(())
}
pub fn ini_path(p: &Profile, name: &str) -> Result<PathBuf, String> {
    if !["Game.ini", "GameUserSettings.ini"].contains(&name) {
        return Err("Fichier INI non autorisé.".into());
    }
    Ok(Path::new(&p.directory)
        .join("ShooterGame/Saved/Config/WindowsServer")
        .join(name))
}
pub fn admin_password(p: &Profile) -> Result<String, String> {
    let text = fs::read_to_string(ini_path(p, "GameUserSettings.ini")?).map_err(|_| {
        "GameUserSettings.ini introuvable. Activer RCON et renseigner ServerAdminPassword."
            .to_string()
    })?;
    let mut section = false;
    let mut password = None;
    let mut enabled = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.eq_ignore_ascii_case("[ServerSettings]");
        }
        if section && let Some((key, value)) = line.split_once('=') {
            if key.trim().eq_ignore_ascii_case("ServerAdminPassword") {
                password = Some(value.trim().to_string());
            }
            if key.trim().eq_ignore_ascii_case("RCONEnabled") {
                enabled = value.trim().eq_ignore_ascii_case("true");
            }
        }
    }
    if !enabled {
        return Err("Activer RCONEnabled=True dans [ServerSettings].".into());
    }
    password
        .filter(|s| !s.is_empty())
        .ok_or("ServerAdminPassword est vide.".into())
}
