use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub const URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
pub fn isolated_runtime(source: &Path, directory: &Path) -> Result<PathBuf, String> {
    validate_directory(directory)?;
    let target = directory.join("steamcmd.exe");
    let marker = directory.join("panel-source.txt");
    let identity = source
        .canonicalize()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    if valid_executable(&target) && fs::read_to_string(&marker).ok().as_deref() == Some(&identity) {
        return Ok(target);
    }
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    fs::copy(source, &target).map_err(|e| e.to_string())?;
    let parent = source.parent().ok_or("Dossier SteamCMD manquant")?;
    // Each profile gets private caches, locks and working files. Never copy steamapps or userdata.
    for name in [
        "steam.dll",
        "steamclient.dll",
        "steamclient64.dll",
        "steamconsole.dll",
        "steamconsole64.dll",
        "tier0_s.dll",
        "tier0_s64.dll",
        "vstdlib_s.dll",
        "vstdlib_s64.dll",
        "crashhandler.dll",
        "crashhandler64.dll",
    ] {
        let file = parent.join(name);
        if file.is_file() {
            crate::manager::copy_tree(&file, &directory.join(name))?;
        }
    }
    for name in ["bin", "public"] {
        let folder = parent.join(name);
        if folder.is_dir() {
            crate::manager::copy_tree(&folder, &directory.join(name))?;
        }
    }
    let package = parent.join("package");
    if package.is_dir() {
        fs::create_dir_all(directory.join("package")).map_err(|e| e.to_string())?;
        for file in fs::read_dir(package).map_err(|e| e.to_string())? {
            let file = file.map_err(|e| e.to_string())?;
            if file
                .path()
                .extension()
                .is_some_and(|x| x == "installed" || x == "manifest")
            {
                crate::manager::copy_tree(
                    &file.path(),
                    &directory.join("package").join(file.file_name()),
                )?;
            }
        }
    }
    fs::write(marker, identity).map_err(|e| e.to_string())?;
    Ok(target)
}
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
    data.join("tools").join("steamcmd").join("steamcmd.exe")
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
pub fn validate_directory(directory: &Path) -> Result<(), String> {
    if !directory.is_absolute()
        || directory.parent().is_none()
        || directory
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        || directory.to_string_lossy().contains('\0')
    {
        return Err("Choisissez un dossier absolu dédié, par exemple D:\\SteamCMD, et non la racine d’un disque.".into());
    }
    let mut current = Some(directory);
    while let Some(path) = current {
        if let Ok(meta) = fs::symlink_metadata(path) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("Choisissez un dossier sans jonction ni lien symbolique.".into());
                }
            }
            if meta.file_type().is_symlink() {
                return Err("Les liens symboliques ne sont pas acceptés.".into());
            }
        }
        current = path.parent();
    }
    Ok(())
}
pub async fn download_to(directory: &Path) -> Result<PathBuf, String> {
    validate_directory(directory)?;
    let target = directory.join("steamcmd.exe");
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
pub fn pick_directory(initial: &str) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        let mut dialog =
            rfd::FileDialog::new().set_title("Choisir un dossier sur la machine Windows hôte");
        let initial = Path::new(initial);
        if let Some(existing) = initial.ancestors().find(|p| p.is_dir()) {
            dialog = dialog.set_directory(existing);
        }
        Ok(dialog.pick_folder())
    }
    #[cfg(not(windows))]
    {
        let _ = initial;
        Err("La sélection native de dossier nécessite Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_runtimes_do_not_share_caches_or_copy_game_data() {
        let root = std::env::temp_dir().join(format!("steam-isolation-{}", uuid::Uuid::new_v4()));
        let source = managed_path(&root);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let mut pe = vec![0u8; 128];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        fs::write(&source, &pe).unwrap();
        for folder in ["steamapps", "userdata", "appcache"] {
            fs::create_dir_all(source.parent().unwrap().join(folder)).unwrap();
            fs::write(
                source.parent().unwrap().join(folder).join("private"),
                b"do not copy",
            )
            .unwrap();
        }
        let first = root.join("first");
        let second = root.join("second");
        assert_ne!(
            isolated_runtime(&source, &first).unwrap(),
            isolated_runtime(&source, &second).unwrap()
        );
        for folder in ["steamapps", "userdata", "appcache"] {
            assert!(!first.join(folder).exists());
            assert!(!second.join(folder).exists());
        }
        fs::create_dir(first.join("appcache")).unwrap();
        fs::write(first.join("appcache/test"), b"one").unwrap();
        isolated_runtime(&source, &first).unwrap();
        assert!(first.join("appcache/test").exists());
        assert!(!second.join("appcache/test").exists());
        fs::remove_dir_all(root).unwrap();
    }
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
