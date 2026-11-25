use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Serialize, Deserialize)]
pub struct JobSpec {
    #[serde(default)]
    pub runtime: Option<PathBuf>,
    pub run_id: String,
    pub kind: String,
    pub steamcmd: PathBuf,
    pub directory: Option<PathBuf>,
    pub log: PathBuf,
    pub result: PathBuf,
    pub visible: bool,
}
pub fn args(kind: &str, directory: Option<&Path>) -> Result<Vec<String>, String> {
    let mut args = vec![
        "+@ShutdownOnFailedCommand".into(),
        "1".into(),
        "+@NoPromptForPassword".into(),
        "1".into(),
    ];
    match kind {
        "init" => {}
        "check" => args.extend(
            [
                "+login",
                "anonymous",
                "+app_info_update",
                "1",
                "+app_info_print",
                "2430930",
            ]
            .map(String::from),
        ),
        "install" | "update" => {
            let directory = directory
                .filter(|p| p.is_absolute())
                .ok_or("Dossier de serveur absolu requis")?;
            args.extend([
                "+force_install_dir".into(),
                directory.to_string_lossy().into(),
                "+login".into(),
                "anonymous".into(),
                "+app_info_update".into(),
                "1".into(),
                "+app_info_print".into(),
                "2430930".into(),
                "+app_update".into(),
                "2430930".into(),
                "validate".into(),
            ]);
        }
        _ => return Err("Type de tâche SteamCMD inconnu.".into()),
    }
    args.push("+quit".into());
    Ok(args)
}
pub fn launch(spec: &JobSpec, spec_file: &Path) -> Result<Child, String> {
    crate::model::save(
        &spec.result,
        &json!({"running":true,"runId":spec.run_id,"kind":spec.kind,"startedAt":crate::now(),"message":"SteamCMD en cours : initialisation / opération demandée."}),
    )?;
    crate::model::save(spec_file, spec)?;
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("--console-job")
        .arg(spec_file)
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(if spec.visible { 0x00000010 } else { 0x08000000 });
    }
    if !spec.visible {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    // Visible workers attach directly to their own Windows console, independent from the hidden panel.
    #[cfg(windows)]
    if spec.visible {
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    }
    match command.spawn() {
        Ok(child) => Ok(child),
        Err(e) => {
            let message = e.to_string();
            let _ = crate::model::save(
                &spec.result,
                &json!({"running":false,"success":false,"kind":spec.kind,"message":message}),
            );
            Err(message)
        }
    }
}
fn forward(
    mut reader: impl Read + Send + 'static,
    log: Arc<Mutex<fs::File>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if let Ok(mut file) = log.lock() {
                let _ = file.write_all(&buffer[..n]);
                let _ = file.flush();
            }
            let _ = std::io::stdout().write_all(&buffer[..n]);
            let _ = std::io::stdout().flush();
        }
    })
}
pub fn tail(path: &Path, limit: u64) -> String {
    use std::io::{Seek, SeekFrom};
    let Ok(mut f) = fs::File::open(path) else {
        return String::new();
    };
    let size = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(size.saturating_sub(limit)));
    let mut bytes = vec![];
    let _ = f.take(limit).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into()
}
pub fn execute(file: &Path) -> Result<(), String> {
    let spec: JobSpec = serde_json::from_slice(&fs::read(file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if spec.visible {
        use std::os::windows::io::IntoRawHandle;
        unsafe extern "system" {
            fn SetStdHandle(which: u32, handle: *mut std::ffi::c_void) -> i32;
            fn SetConsoleOutputCP(code: u32) -> i32;
        }
        if let Ok(output) = fs::OpenOptions::new().write(true).open("CONOUT$") {
            let handle = output.into_raw_handle();
            unsafe {
                SetStdHandle(-11i32 as u32, handle);
                SetStdHandle(-12i32 as u32, handle);
                SetConsoleOutputCP(65001);
            }
        }
    }
    let outcome = execute_job(&spec);
    let mut result = match outcome {
        Ok(v) => v,
        Err(error) => json!({"success":false,"message":error}),
    };
    result["running"] = json!(false);
    result["runId"] = json!(spec.run_id);
    result["kind"] = json!(spec.kind);
    result["finishedAt"] = json!(crate::now());
    crate::model::save(&spec.result, &result)?;
    let summary = format!(
        "\n=== {} ===\n",
        result["message"].as_str().unwrap_or("Tâche terminée")
    );
    print!("{summary}");
    if let Ok(mut log) = fs::OpenOptions::new().append(true).open(&spec.log) {
        let _ = log.write_all(summary.as_bytes());
    }
    if spec.visible {
        println!(
            "Opération terminée. Appuyez sur Entrée pour fermer cette fenêtre.\nLe résultat et le journal restent disponibles dans le panneau web."
        );
        #[cfg(windows)]
        if let Ok(input) = fs::File::open("CONIN$") {
            let _ = std::io::BufReader::new(input).read_line(&mut String::new());
        }
    }
    Ok(())
}
fn execute_job(spec: &JobSpec) -> Result<Value, String> {
    if !crate::steamcmd::valid_executable(&spec.steamcmd) {
        return Err("SteamCMD est introuvable ou invalide.".into());
    }
    let steamcmd = if let Some(runtime) = &spec.runtime {
        crate::steamcmd::isolated_runtime(&spec.steamcmd, runtime)?
    } else {
        spec.steamcmd.clone()
    };
    let args = args(&spec.kind, spec.directory.as_deref())?;
    fs::create_dir_all(spec.log.parent().ok_or("Dossier de journal manquant")?)
        .map_err(|e| e.to_string())?;
    let mut output = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&spec.log)
        .map_err(|e| e.to_string())?;
    let header = format!(
        "ARK Server Panel — SteamCMD {}\nProgramme : {}\nArguments : {}\n\n",
        spec.kind,
        steamcmd.display(),
        args.join(" ")
    );
    let _ = output.write_all(header.as_bytes());
    print!("{header}");
    let log = Arc::new(Mutex::new(output));
    let started = Instant::now();
    let mut retries = 0;
    let mut configuration_retry = false;
    let status = loop {
        let attempt_start = fs::metadata(&spec.log).map(|m| m.len()).unwrap_or(0);
        let mut command = Command::new(&steamcmd);
        command
            .args(&args)
            .current_dir(steamcmd.parent().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        if !spec.visible {
            crate::manager::hidden(&mut command);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("Impossible de lancer SteamCMD : {e}"))?;
        let stdout = forward(child.stdout.take().unwrap(), log.clone());
        let stderr = forward(child.stderr.take().unwrap(), log.clone());
        let timeout = Duration::from_secs(if spec.kind == "check" {
            180
        } else if spec.kind == "init" {
            900
        } else {
            86400
        });
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if started.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout.join();
                let _ = stderr.join();
                return Err("Délai SteamCMD dépassé. Consulter le terminal.".into());
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        let _ = stdout.join();
        let _ = stderr.join();
        let attempt_bytes = fs::metadata(&spec.log)
            .map(|m| m.len().saturating_sub(attempt_start))
            .unwrap_or(0);
        let transcript = tail(&spec.log, attempt_bytes.min(2 * 1024 * 1024));
        if status.code() == Some(7)
            && retries < 2
            && (spec.kind == "init" || transcript.contains("Update complete, launching"))
        {
            retries += 1;
            let message = "\nSteamCMD mis à jour : redémarrage de son initialisation.\n";
            print!("{message}");
            if let Ok(mut output) = log.lock() {
                let _ = output.write_all(message.as_bytes());
            }
            continue;
        }
        if matches!(spec.kind.as_str(), "install" | "update")
            && transcript.contains("Missing configuration")
        {
            if configuration_retry {
                return Err("SteamCMD ne trouve toujours pas la configuration ASA après rafraîchissement et deux tentatives. Aucun succès d’installation n’est confirmé. Consultez le terminal et les journaux SteamCMD appinfo_log.txt / content_log.txt.".into());
            }
            configuration_retry = true;
            let message = "\nSteamCMD : configuration ASA manquante. Nouvelle tentative unique avec rafraîchissement des informations Steam.\n";
            print!("{message}");
            if let Ok(mut output) = log.lock() {
                let _ = output.write_all(message.as_bytes());
            }
            continue;
        }
        break status;
    };
    if !status.success() {
        return Err(format!(
            "SteamCMD a échoué (code {:?}). Consulter le terminal.",
            status.code()
        ));
    }
    if spec.kind == "check" {
        return Ok(
            json!({"success":true,"message":"Dernier build public ASA récupéré.","publicBuild":crate::versions::public_build(&tail(&spec.log,2*1024*1024))?}),
        );
    }
    if let Some(dir) = &spec.directory {
        let installed = crate::versions::installed(dir);
        if installed["buildId"].is_null() || installed["complete"] != true {
            return Err("SteamCMD est terminé mais le manifeste ne confirme pas une installation complète. Consulter le terminal et revérifier le serveur.".into());
        }
        return Ok(
            json!({"success":true,"message":"Installation / mise à jour terminée. Vérifiez la version avant le lancement.","installed":installed}),
        );
    }
    Ok(
        json!({"success":true,"message":"SteamCMD installé et initialisé. Vous pouvez vérifier ou installer un serveur ASA."}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anonymous_server_arguments_are_distinct_from_readonly_version_check() {
        let dir = std::env::temp_dir().join("ASA test");
        let check = args("check", None).unwrap();
        assert!(!check.iter().any(|a| a == "+app_update"));
        assert!(check.iter().any(|a| a == "anonymous"));
        let install = args("install", Some(&dir)).unwrap();
        assert!(
            install.iter().position(|a| a == "+app_info_print").unwrap()
                < install.iter().position(|a| a == "+app_update").unwrap()
        );
        assert!(install.iter().any(|a| a == "2430930"));
        assert!(!install.iter().any(|a| a == "2399830"));
        assert!(install.iter().any(|a| a == &dir.to_string_lossy()));
    }
}
