use crate::{manager::hash_tree, model::*, server::backup_operation, updates::*};
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("ark-panel-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn profile(t: &Temp) -> Profile {
    Profile {
        id: "test-profile".into(),
        name: "Île des survivants".into(),
        directory: t.0.join("serveur avec espaces").to_string_lossy().into(),
        map: "TheIsland_WP".into(),
        game_port: 7777,
        rcon_port: 27020,
        mods: "123456,789012".into(),
    }
}
#[test]
fn rejects_conflicting_ports_and_overlapping_paths() {
    let t = Temp::new();
    let p = profile(&t);
    assert!(validate_profile(&p, &[], &t.0.join("data")).is_ok());
    let mut other = p.clone();
    other.id = "second".into();
    other.directory = t.0.join("second").to_string_lossy().into();
    assert!(validate_profile(&other, &[p.clone()], &t.0.join("data")).is_err());
    other.game_port = 7787;
    other.rcon_port = 27030;
    other.directory = PathBuf::from(&p.directory)
        .join("nested")
        .to_string_lossy()
        .into();
    assert!(validate_profile(&other, &[p], &t.0.join("data")).is_err());
}
#[test]
fn rejects_untrusted_repo_and_manifest() {
    assert!(validate_repo("oxo140/ark-server-panel", "main").is_ok());
    for repo in [
        "https://github.com/a/b",
        "a/b/c",
        "a/../b",
        "user:secret@github.com/a",
    ] {
        assert!(validate_repo(repo, "main").is_err());
    }
    let mut manifest = Manifest {
        name: "ark-server-panel".into(),
        version: "1.0.0".into(),
        source_sha: "a".repeat(40),
        asset: "ark-server-panel-windows-x64.exe".into(),
        size: 42,
        sha256: "b".repeat(64),
    };
    assert!(validate_manifest(&manifest).is_ok());
    manifest.asset = "../../bad.exe".into();
    assert!(validate_manifest(&manifest).is_err());
}
#[test]
fn ini_roundtrip_preserves_duplicate_unknown_and_unicode() {
    let t = Temp::new();
    let file = t.0.join("Game.ini");
    let bytes="[/Script/ShooterGame.ShooterGameMode]\r\n; Français\r\nOverrideEngramEntries=A\r\nOverrideEngramEntries=B\r\nUnknownModSetting=île\r\n".as_bytes();
    save_bytes(&file, bytes).unwrap();
    assert_eq!(fs::read(&file).unwrap(), bytes);
    save_bytes(&file, b"replacement").unwrap();
    assert_eq!(fs::read(file).unwrap(), b"replacement");
}
#[test]
fn backup_restore_checks_hashes_and_keeps_previous_save() {
    let t = Temp::new();
    let p = profile(&t);
    let data = t.0.join("data");
    let saved = PathBuf::from(&p.directory).join("ShooterGame/Saved");
    fs::create_dir_all(saved.join("SavedArks/TheIsland_WP")).unwrap();
    fs::write(
        saved.join("SavedArks/TheIsland_WP/world.ark"),
        b"original-world",
    )
    .unwrap();
    backup_operation(&p, &data, None).unwrap();
    let base = data.join("backups").join(&p.id);
    let backup = fs::read_dir(&base)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .to_string();
    fs::write(saved.join("SavedArks/TheIsland_WP/world.ark"), b"new-world").unwrap();
    backup_operation(&p, &data, Some(&backup)).unwrap();
    assert_eq!(
        fs::read(saved.join("SavedArks/TheIsland_WP/world.ark")).unwrap(),
        b"original-world"
    );
    assert!(
        fs::read_dir(saved.parent().unwrap())
            .unwrap()
            .flatten()
            .any(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with("Saved-before-restore-"))
    );
    fs::write(
        base.join(&backup)
            .join("Saved/SavedArks/TheIsland_WP/world.ark"),
        b"corrupted",
    )
    .unwrap();
    assert!(backup_operation(&p, &data, Some(&backup)).is_err());
    assert_eq!(hash_tree(&saved).unwrap().len(), 1);
}
#[test]
fn malformed_data_is_never_silently_reset() {
    let t = Temp::new();
    let file = t.0.join("settings.json");
    fs::write(&file, b"broken").unwrap();
    assert!(load::<Settings>(&file).is_err());
}
#[test]
fn rcon_password_only_reads_correct_section() {
    let t = Temp::new();
    let p = profile(&t);
    let file = ini_path(&p, "GameUserSettings.ini").unwrap();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file,"[Other]\nServerAdminPassword=wrong\n[ServerSettings]\nRCONEnabled=True\nServerAdminPassword=test-only\n").unwrap();
    assert_eq!(admin_password(&p).unwrap(), "test-only");
}
#[tokio::test]
async fn rcon_handles_fragmented_auth_and_reply() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mock = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let n = stream.read_i32_le().await.unwrap();
        let mut auth = vec![0; n as usize];
        stream.read_exact(&mut auth).await.unwrap();
        assert!(String::from_utf8_lossy(&auth).contains("test-only"));
        let response = crate::rcon::packet(1, 2, "");
        stream.write_all(&response[..3]).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        stream.write_all(&response[3..]).await.unwrap();
        let n = stream.read_i32_le().await.unwrap();
        let mut request = vec![0; n as usize];
        stream.read_exact(&mut request).await.unwrap();
        let response = crate::rcon::packet(2, 0, "No Players Connected");
        stream.write_all(&response).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    });
    assert_eq!(
        crate::rcon::command(port, "test-only", "ListPlayers")
            .await
            .unwrap(),
        "No Players Connected"
    );
    mock.await.unwrap();
}
