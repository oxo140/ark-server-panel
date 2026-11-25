use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn entry() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA indisponible")?)
            .join("Microsoft/Windows/Start Menu/Programs/Startup/ArkServerPanel.vbs"),
    )
}
const MARKER: &str = "' ArkServerPanel managed startup";
pub fn status() -> Value {
    match entry() {
        Ok(p) => {
            json!({"enabled":read_script(&p).is_ok_and(|s|s.starts_with(MARKER)),"supported":cfg!(windows)})
        }
        Err(e) => json!({"enabled":false,"supported":false,"error":e}),
    }
}
fn quote(s: &str) -> String {
    s.replace('"', "\"\"")
}
pub fn configure(data: &Path, enabled: bool) -> Result<Value, String> {
    let p = entry()?;
    if let Ok(old) = read_script(&p)
        && !old.starts_with(MARKER)
    {
        return Err("Un fichier de démarrage non géré occupe cet emplacement.".into());
    }
    if enabled {
        let exe = std::env::var_os("ARK_PANEL_LAUNCHER")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_exe().map_err(|e| e.to_string())?);
        let mut text = format!(
            "{MARKER}\r\nSet shell = CreateObject(\"WScript.Shell\")\r\nSet env = shell.Environment(\"PROCESS\")\r\n"
        );
        for (k, v) in [
            ("ARK_PANEL_DATA", data.to_string_lossy().into_owned()),
            (
                "ARK_PANEL_BIND",
                std::env::var("ARK_PANEL_BIND").unwrap_or("127.0.0.1".into()),
            ),
            (
                "ARK_PANEL_PORT",
                std::env::var("ARK_PANEL_PORT").unwrap_or("3080".into()),
            ),
        ] {
            text += &format!("env(\"{k}\") = \"{}\"\r\n", quote(&v));
        }
        text += &format!(
            "shell.Run Chr(34) & \"{}\" & Chr(34) & \" --background\", 0, False\r\n",
            quote(&exe.to_string_lossy())
        );
        fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
        // WScript interprets Unicode scripts reliably with a UTF-16 BOM.
        let bytes: Vec<u8> = std::iter::once(0xfeffu16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        fs::write(&p, bytes).map_err(|e| e.to_string())?;
    } else if p.exists() {
        fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(
        json!({"enabled":enabled,"supported":true,"message":if enabled{"Démarrage automatique activé à l’ouverture de session Windows."}else{"Démarrage automatique désactivé."}}),
    )
}

fn read_script(p: &Path) -> std::io::Result<String> {
    let bytes = fs::read(p)?;
    if bytes.starts_with(&[255, 254]) {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    } else {
        Ok(String::from_utf8_lossy(&bytes).into())
    }
}
