use crate::model::{save, validate_repo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REVISION: &str = env!("ARK_BUILD_SHA");
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub checked_at: Option<u64>,
    pub available: bool,
    pub current_version: String,
    pub current_sha: String,
    pub latest_version: Option<String>,
    pub latest_sha: Option<String>,
    pub branch_sha: Option<String>,
    pub error: Option<String>,
    pub pending: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub source_sha: String,
    pub asset: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Pending {
    pub executable: PathBuf,
    pub sha: String,
}
pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("ark-server-panel/1.0")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}
async fn json_url(url: &str) -> Result<serde_json::Value, String> {
    let response = client()?
        .get(url)
        .send()
        .await
        .map_err(|_| "GitHub inaccessible : vérifier le réseau.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "GitHub HTTP {} : vérifier le dépôt public, la publication et le quota API.",
            response.status()
        ));
    }
    if response.content_length().unwrap_or(0) > 2 * 1024 * 1024 {
        return Err("Réponse GitHub trop volumineuse.".into());
    }
    response
        .json()
        .await
        .map_err(|_| "Réponse GitHub invalide.".into())
}
pub fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn valid_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn validate_manifest(m: &Manifest) -> Result<(), String> {
    if m.name != "ark-server-panel"
        || m.asset != "ark-server-panel-windows-x64.exe"
        || !valid_sha(&m.source_sha)
        || !valid_hash(&m.sha256)
        || m.size == 0
        || m.size > 200 * 1024 * 1024
        || m.version.is_empty()
    {
        return Err("Manifeste de publication invalide.".into());
    }
    Ok(())
}
async fn release(repo: &str) -> Result<(Manifest, String), String> {
    validate_repo(repo, "main")?;
    let release = json_url(&format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ))
    .await?;
    let tag = release["tag_name"].as_str().ok_or("Publication sans tag")?;
    if !tag
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
    {
        return Err("Tag de publication invalide.".into());
    }
    let assets = release["assets"]
        .as_array()
        .ok_or("Publication sans fichiers")?;
    let find = |name: &str| -> Result<String, String> {
        let url = assets
            .iter()
            .find(|a| a["name"] == name)
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or_else(|| format!("Fichier de publication absent : {name}"))?;
        if !url.starts_with(&format!("https://github.com/{repo}/releases/download/")) {
            return Err("Adresse de téléchargement inattendue.".into());
        }
        Ok(url.into())
    };
    let manifest: Manifest = serde_json::from_value(json_url(&find("release.json")?).await?)
        .map_err(|_| "Manifeste invalide".to_string())?;
    validate_manifest(&manifest)?;
    if tag != format!("v{}", manifest.version) {
        return Err("Version et tag GitHub différents.".into());
    }
    let commit = json_url(&format!(
        "https://api.github.com/repos/{repo}/commits/{tag}"
    ))
    .await?;
    if commit["sha"].as_str() != Some(&manifest.source_sha) {
        return Err("Le manifeste ne correspond pas au commit du tag.".into());
    }
    let url = find(&manifest.asset)?;
    Ok((manifest, url))
}
pub async fn check(repo: &str) -> Result<UpdateStatus, String> {
    validate_repo(repo, "main")?;
    let head = json_url(&format!("https://api.github.com/repos/{repo}/commits/main")).await?;
    let branch_sha = head["sha"]
        .as_str()
        .filter(|s| valid_sha(s))
        .ok_or("Commit main invalide")?
        .to_string();
    let (manifest, _) = release(repo).await?;
    Ok(UpdateStatus {
        checked_at: Some(crate::now()),
        available: manifest.source_sha != REVISION,
        current_version: VERSION.into(),
        current_sha: REVISION.into(),
        latest_version: Some(manifest.version),
        latest_sha: Some(manifest.source_sha),
        branch_sha: Some(branch_sha),
        error: None,
        pending: false,
    })
}
pub async fn prepare(repo: &str, dir: &Path) -> Result<Pending, String> {
    let (manifest, url) = release(repo).await?;
    if manifest.source_sha == REVISION {
        return Err("Version déjà installée.".into());
    }
    let stage = dir.join("versions").join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let file = stage.join("ark-server-panel.exe");
    let temporary = stage.join("download.tmp");
    let mut response = client()?
        .get(url)
        .send()
        .await
        .map_err(|_| "Téléchargement GitHub impossible.".to_string())?
        .error_for_status()
        .map_err(|_| "Téléchargement GitHub refusé.".to_string())?;
    let mut output = tokio::fs::File::create(&temporary)
        .await
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut size = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Téléchargement interrompu.".to_string())?
    {
        size += chunk.len() as u64;
        if size > manifest.size {
            return Err("Taille du téléchargement incorrecte.".into());
        }
        hash.update(&chunk);
        output.write_all(&chunk).await.map_err(|e| e.to_string())?;
    }
    output.flush().await.map_err(|e| e.to_string())?;
    drop(output);
    if size != manifest.size || format!("{:x}", hash.finalize()) != manifest.sha256.to_lowercase() {
        return Err("Échec du contrôle SHA-256. Mise à jour non activée.".into());
    }
    fs::rename(&temporary, &file).map_err(|e| e.to_string())?;
    let pending = Pending {
        executable: file,
        sha: manifest.source_sha,
    };
    save(&dir.join("pending-update.json"), &pending)?;
    Ok(pending)
}
