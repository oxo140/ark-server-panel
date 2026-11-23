use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub const URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
pub fn valid_executable(path: &Path) -> bool {
    if !path.is_absolute()
        || !path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("steamcmd.exe"))
    {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut dos = [0; 64];
    if file.read_exact(&mut dos).is_err() || &dos[..2] != b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if offset > 16 * 1024 * 1024 || file.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    let mut signature = [0; 4];
    file.read_exact(&mut signature).is_ok() && signature == *b"PE\0\0"
}
pub fn managed_path(data: &Path) -> PathBuf {
    data.join("tools/steamcmd/steamcmd.exe")
}
pub fn status(configured: &str, data: &Path) -> Value {
    let mut candidates = vec![];
    if !configured.trim().is_empty() {
        candidates.push(PathBuf::from(configured.trim()));
    }
    candidates.push(managed_path(data));
    candidates.push(PathBuf::from("C:\\SteamCMD\\steamcmd.exe"));
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join("steamcmd/steamcmd.exe"));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            candidates.push(dir.join("steamcmd.exe"));
        }
    }
    let mut unique = vec![];
    for p in candidates {
        if !unique.iter().any(|prior: &PathBuf| {
            prior
                .to_string_lossy()
                .eq_ignore_ascii_case(&p.to_string_lossy())
        }) {
            unique.push(p);
        }
    }
    let found = unique.iter().find(|p| valid_executable(p));
    json!({"ready":found.is_some(),"path":found,"configured":configured,"configuredValid":!configured.is_empty()&&valid_executable(Path::new(configured)),"managedPath":managed_path(data),"downloadUrl":URL,"message":if found.is_some(){"SteamCMD détecté. Son chemin sera vérifié de nouveau avant lancement."}else{"SteamCMD est absent ou le fichier configuré n’est pas un exécutable Windows valide."}})
}
pub fn extract_bootstrap(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| "L’archive SteamCMD reçue est invalide.".to_string())?;
    if archive.len() > 100 {
        return Err("Archive SteamCMD inattendue.".into());
    }
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // Extract only this exact root member, never paths from the network archive.
        if entry.name().eq_ignore_ascii_case("steamcmd.exe") && !entry.is_dir() {
            if entry.size() > 20 * 1024 * 1024 {
                return Err("Exécutable SteamCMD trop volumineux.".into());
            }
            let mut output = vec![];
            entry
                .take(20 * 1024 * 1024 + 1)
                .read_to_end(&mut output)
                .map_err(|e| e.to_string())?;
            if output.len() > 20 * 1024 * 1024 {
                return Err("Exécutable SteamCMD trop volumineux.".into());
            }
            return Ok(output);
        }
    }
    Err("steamcmd.exe est absent de l’archive officielle.".into())
}
pub async fn download(data: &Path) -> Result<PathBuf, String> {
    let target = managed_path(data);
    if valid_executable(&target) {
        return Ok(target);
    }
    let mut response = crate::updates::client()?
        .get(URL)
        .send()
        .await
        .map_err(|_| {
            "Téléchargement Valve impossible. Vérifier la connexion Internet.".to_string()
        })?
        .error_for_status()
        .map_err(|_| "Le serveur de téléchargement Valve refuse la requête.".to_string())?;
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Téléchargement SteamCMD interrompu.".to_string())?
    {
        if bytes.len() + chunk.len() > 20 * 1024 * 1024 {
            return Err("Archive SteamCMD trop volumineuse.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let executable = extract_bootstrap(&bytes)?;
    fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    let stage = target
        .parent()
        .unwrap()
        .join(format!("download-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let staged = stage.join("steamcmd.exe");
    fs::write(&staged, &executable).map_err(|e| e.to_string())?;
    if !valid_executable(&staged) {
        return Err("Le fichier téléchargé n’est pas un exécutable SteamCMD valide.".into());
    }
    if target.exists() {
        fs::rename(
            &target,
            target.with_extension(format!("exe.{}.bak", uuid::Uuid::new_v4())),
        )
        .map_err(|e| e.to_string())?;
    }
    fs::rename(&staged, &target).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir(stage);
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_install_is_found_when_the_configured_path_is_invalid() {
        let root = std::env::temp_dir().join(format!("steamcmd-fixture-{}", uuid::Uuid::new_v4()));
        let path = managed_path(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut pe = vec![0u8; 128];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        fs::write(&path, &pe).unwrap();
        assert!(valid_executable(&path));
        let invalid = root.join("missing/steamcmd.exe");
        let found = status(&invalid.to_string_lossy(), &root);
        assert_eq!(found["configuredValid"], false);
        assert_eq!(found["path"], path.to_string_lossy().as_ref());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn zip_paths_cannot_escape_and_missing_executable_is_rejected() {
        use std::io::Write;
        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        zip.start_file("../steamcmd.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"MZ").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        assert!(extract_bootstrap(&bytes).is_err());
        assert!(!valid_executable(Path::new("steamcmd.exe")));
    }
}
