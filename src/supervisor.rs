use crate::{
    manager::hidden,
    model::{data_dir, save},
    updates::Pending,
};
use std::{
    fs,
    process::{Command, Stdio},
};

fn read_pending(path: &std::path::Path) -> Option<Pending> {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}
pub async fn supervise() -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let original = std::env::current_exe().map_err(|e| e.to_string())?;
    let active_file = dir.join("active-version.json");
    let pending_file = dir.join("pending-update.json");
    let mut current = read_pending(&active_file)
        .filter(|p| p.executable.is_file())
        .unwrap_or(Pending {
            executable: original,
            sha: crate::updates::REVISION.into(),
        });
    let bind = std::env::var("ARK_PANEL_BIND").unwrap_or("127.0.0.1".into());
    let port = std::env::var("ARK_PANEL_PORT").unwrap_or("3080".into());
    loop {
        let candidate = read_pending(&pending_file);
        if let Some(pending) = &candidate {
            let versions = dir
                .join("versions")
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let exe = pending
                .executable
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !exe.starts_with(&versions) {
                return Err("Chemin de mise à jour refusé.".into());
            }
        }
        let target = candidate.as_ref().unwrap_or(&current);
        let boot = uuid::Uuid::new_v4().to_string();
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("panel.log"))
            .map_err(|e| e.to_string())?;
        let spawn_result = hidden(
            Command::new(&target.executable)
                .arg("--serve")
                .env(
                    "ARK_PANEL_LAUNCHER",
                    std::env::current_exe().map_err(|e| e.to_string())?,
                )
                .env("ARK_PANEL_BOOT", &boot),
        )
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .spawn();
        let mut child = match spawn_result {
            Ok(child) => child,
            Err(_) if candidate.is_some() => {
                fs::remove_file(&pending_file).map_err(|e| e.to_string())?;
                save(
                    &dir.join("last-update.json"),
                    &serde_json::json!({"success":false,"message":"Le fichier de mise à jour ne peut pas être exécuté. Retour à la version précédente."}),
                )?;
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        let client = crate::updates::client()?;
        let mut ready = false;
        for _ in 0..80 {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                break;
            }
            if let Ok(response) = client
                .get(format!("http://{bind}:{port}/health"))
                .timeout(std::time::Duration::from_millis(400))
                .send()
                .await
                && let Ok(value) = response.json::<serde_json::Value>().await
                && value["boot"].as_str() == Some(&boot)
                && value["revision"].as_str() == Some(&target.sha)
            {
                ready = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        if !ready {
            let _ = child.kill();
            let _ = child.wait();
            if candidate.is_some() {
                fs::remove_file(&pending_file).map_err(|e| e.to_string())?;
                save(
                    &dir.join("last-update.json"),
                    &serde_json::json!({"success":false,"message":"Démarrage de la nouvelle version impossible. Retour à la version précédente."}),
                )?;
                continue;
            }
            return Err(format!(
                "Le panneau ne démarre pas. Consulter {}",
                dir.join("panel.log").display()
            ));
        }
        if let Some(pending) = candidate {
            current = pending;
            save(&active_file, &current)?;
            fs::remove_file(&pending_file).map_err(|e| e.to_string())?;
            save(
                &dir.join("last-update.json"),
                &serde_json::json!({"success":true,"sha":current.sha}),
            )?;
        }
        let token = fs::read_to_string(dir.join("access-key.txt")).map_err(|e| e.to_string())?;
        let url = format!("http://{bind}:{port}/#{}", token.trim());
        println!("Panneau prêt : http://{bind}:{port}/");
        if std::env::var("ARK_PANEL_NO_BROWSER").is_err()
            && !std::env::args().any(|a| a == "--background")
        {
            open_browser(&url);
        }
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        if status.code() != Some(75) {
            return Ok(());
        }
    }
}
pub fn open_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ =
            hidden(Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", url])).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}
