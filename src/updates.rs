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
        if response
            .headers()
            .get("x-ratelimit-remaining")
            .is_some_and(|v| v == "0")
        {
            return Err(format!(
                "Quota API GitHub épuisé pour cette connexion. Réinitialisation Unix : {}. Le dépôt et vos serveurs ne sont pas en cause.",
                response
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("inconnue")
            ));
        }
        return Err(format!(
            "GitHub HTTP {} : accès au fichier de publication refusé ou fichier indisponible.",
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
fn parse_refs(bytes: &[u8]) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut at = 0;
    let mut refs = std::collections::BTreeMap::new();
    while at < bytes.len() {
        let prefix = bytes.get(at..at + 4).ok_or("Références Git tronquées")?;
        let len = usize::from_str_radix(
            std::str::from_utf8(prefix).map_err(|_| "Références Git invalides")?,
            16,
        )
        .map_err(|_| "Références Git invalides")?;
        at += 4;
        if len == 0 {
            continue;
        }
        if len < 4 {
            return Err("Paquet Git invalide".into());
        }
        let packet = bytes.get(at..at + len - 4).ok_or("Paquet Git tronqué")?;
        at += len - 4;
        let line = std::str::from_utf8(packet)
            .map_err(|_| "Références Git invalides")?
            .split('\0')
            .next()
            .unwrap_or("");
        let mut parts = line.split_whitespace();
        if let (Some(sha), Some(name)) = (parts.next(), parts.next())
            && valid_sha(sha)
            && name.starts_with("refs/")
        {
            refs.insert(name.into(), sha.into());
        }
    }
    if refs.is_empty() {
        return Err("GitHub n’a annoncé aucune référence Git.".into());
    }
    Ok(refs)
}
async fn git_refs(repo: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut response = client()?
        .get(format!(
            "https://github.com/{repo}.git/info/refs?service=git-upload-pack"
        ))
        .send()
        .await
        .map_err(|_| "Références GitHub inaccessibles")?
        .error_for_status()
        .map_err(|_| "Accès au dépôt GitHub refusé")?;
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Lecture GitHub interrompue")?
    {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err("Références Git trop volumineuses".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_refs(&bytes)
}
async fn release_with_branch(repo: &str) -> Result<(Manifest, String, String), String> {
    validate_repo(repo, "main")?;
    // Public release redirects and Git's reference advertisement do not consume REST API quota.
    let response = client()?
        .get(format!("https://github.com/{repo}/releases/latest"))
        .send()
        .await
        .map_err(|_| "GitHub inaccessible : vérifier le réseau.")?
        .error_for_status()
        .map_err(|_| "Dernière publication GitHub inaccessible.")?;
    let prefix = format!("https://github.com/{repo}/releases/tag/");
    let tag = response
        .url()
        .as_str()
        .strip_prefix(&prefix)
        .ok_or("Redirection de publication inattendue")?
        .to_string();
    if tag.is_empty()
        || !tag
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
    {
        return Err("Tag de publication invalide.".into());
    }
    let base = format!("https://github.com/{repo}/releases/download/{tag}");
    let manifest_url = format!("{base}/release.json");
    let (manifest, refs) = tokio::try_join!(json_url(&manifest_url), git_refs(repo))?;
    let manifest: Manifest =
        serde_json::from_value(manifest).map_err(|_| "Manifeste invalide".to_string())?;
    validate_manifest(&manifest)?;
    if tag != format!("v{}", manifest.version) {
        return Err("Version et tag GitHub différents.".into());
    }
    let tag_ref = format!("refs/tags/{tag}");
    let commit = refs
        .get(&format!("{tag_ref}^{{}}"))
        .or_else(|| refs.get(&tag_ref));
    if commit != Some(&manifest.source_sha) {
        return Err("Le manifeste ne correspond pas au commit du tag.".into());
    }
    let branch = refs
        .get("refs/heads/main")
        .ok_or("Branche main absente")?
        .clone();
    let url = format!("{base}/{}", manifest.asset);
    Ok((manifest, url, branch))
}
async fn release(repo: &str) -> Result<(Manifest, String), String> {
    let (m, u, _) = release_with_branch(repo).await?;
    Ok((m, u))
}
pub async fn check(repo: &str) -> Result<UpdateStatus, String> {
    validate_repo(repo, "main")?;
    let (manifest, _, branch_sha) = release_with_branch(repo).await?;
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

#[cfg(test)]
mod public_release_tests {
    use super::*;
    #[test]
    fn advertised_refs_keep_main_lightweight_and_peeled_tags() {
        fn packet(s: &str) -> String {
            format!("{:04x}{s}", s.len() + 4)
        }
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let c = "c".repeat(40);
        let wire = format!(
            "{}0000{}{}{}0000",
            packet("# service=git-upload-pack\n"),
            packet(&format!("{a} refs/heads/main\0multi_ack\n")),
            packet(&format!("{b} refs/tags/v1.5.0\n")),
            packet(&format!("{c} refs/tags/v1.5.0^{{}}\n"))
        );
        let refs = parse_refs(wire.as_bytes()).unwrap();
        assert_eq!(refs["refs/heads/main"], a);
        assert_eq!(refs["refs/tags/v1.5.0"], b);
        assert_eq!(refs["refs/tags/v1.5.0^{}"], c);
        assert!(parse_refs(b"fffftruncated").is_err());
        assert!(parse_refs(b"0001").is_err());
        assert!(parse_refs(b"0000").is_err());
    }
}
