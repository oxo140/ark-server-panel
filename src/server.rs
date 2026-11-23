use crate::{
    manager::{Manager, copy_tree, hash_tree},
    model::{self, Profile, Settings},
    updates::{self, UpdateStatus},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, Notify};

pub struct Core {
    settings: Settings,
    manager: Manager,
    operations: HashMap<String, Value>,
    updates: UpdateStatus,
}
pub struct App {
    core: Mutex<Core>,
    data: PathBuf,
    token: String,
    host: String,
    shutdown: Notify,
    update_busy: AtomicBool,
}
type Shared = Arc<App>;
fn failure(message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error":message.to_string()})),
    )
}
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("Champ manquant : {name}"))
}
fn equal(a: &str, b: &str) -> bool {
    use sha2::{Digest, Sha256};
    let a = Sha256::digest(a.as_bytes());
    let b = Sha256::digest(b.as_bytes());
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
fn authorized(app: &App, headers: &HeaderMap) -> Result<(), String> {
    if headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&app.host) {
        return Err("Adresse du panneau refusée.".into());
    }
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(&format!("http://{}", app.host))
    {
        return Err("Origine de requête refusée.".into());
    }
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !equal(bearer, &app.token) {
        return Err("Clé d’accès invalide. Ouvrir le panneau avec son raccourci Windows.".into());
    }
    Ok(())
}
fn secure(response: impl IntoResponse, content_type: Option<&'static str>) -> Response {
    let mut response = response.into_response();
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'".parse().unwrap());
    if let Some(value) = content_type {
        headers.insert(header::CONTENT_TYPE, value.parse().unwrap());
    }
    response
}
async fn api(State(app): State<Shared>, headers: HeaderMap, Json(input): Json<Value>) -> Response {
    if let Err(e) = authorized(&app, &headers) {
        return secure((StatusCode::UNAUTHORIZED, Json(json!({"error":e}))), None);
    }
    let result = dispatch(app, input).await.map(Json).map_err(failure);
    secure(result, None)
}
pub fn router(app: Shared) -> Router {
    Router::new()
        .route("/",get(||async{secure(Html(include_str!("../web/index.html")),None)}))
        .route("/app.js",get(||async{secure(include_str!("../web/app.js"),Some("application/javascript; charset=utf-8"))}))
        .route("/style.css",get(||async{secure(include_str!("../web/style.css"),Some("text/css; charset=utf-8"))}))
        .route("/health",get(||async{secure(Json(json!({"ok":true,"revision":updates::REVISION,"boot":std::env::var("ARK_PANEL_BOOT").unwrap_or_default()})),None)}))
        .route("/api",post(api)).layer(DefaultBodyLimit::max(2*1024*1024)).with_state(app)
}
pub async fn serve() -> Result<(), String> {
    let bind = std::env::var("ARK_PANEL_BIND").unwrap_or("127.0.0.1".into());
    let address: std::net::Ipv4Addr = bind
        .parse()
        .map_err(|_| "ARK_PANEL_BIND doit être une IPv4 explicite.".to_string())?;
    if !address.is_loopback()
        && !(address.octets()[0] == 100 && (64..=127).contains(&address.octets()[1]))
    {
        return Err(
            "La V1 écoute uniquement sur loopback ou une IP Tailscale 100.64.0.0/10.".into(),
        );
    }
    let port = std::env::var("ARK_PANEL_PORT")
        .unwrap_or("3080".into())
        .parse::<u16>()
        .map_err(|e| e.to_string())?;
    let listener = tokio::net::TcpListener::bind((address, port))
        .await
        .map_err(|e| format!("Impossible d’ouvrir le panneau : {e}"))?;
    let data = model::data_dir();
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let key = data.join("access-key.txt");
    let token = match fs::read_to_string(&key) {
        Ok(t) => t.trim().to_string(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let t = format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            );
            fs::write(&key, &t).map_err(|e| e.to_string())?;
            t
        }
        Err(e) => return Err(e.to_string()),
    };
    if token.len() < 32 {
        return Err("Clé d’accès locale invalide.".into());
    }
    let settings: Settings = model::load(&data.join("settings.json"))?;
    let app = Arc::new(App {
        core: Mutex::new(Core {
            settings,
            manager: Manager::new(data.clone())?,
            operations: HashMap::new(),
            updates: UpdateStatus {
                current_version: updates::VERSION.into(),
                current_sha: updates::REVISION.into(),
                ..Default::default()
            },
        }),
        data,
        token,
        host: format!("{address}:{port}"),
        shutdown: Notify::new(),
        update_busy: AtomicBool::new(false),
    });
    let background = app.clone();
    tokio::spawn(async move {
        loop {
            let _ = update_check(&background).await;
            tokio::time::sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
        }
    });
    let shutdown = app.clone();
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async move {
            shutdown.shutdown.notified().await;
        })
        .await
        .map_err(|e| e.to_string())?;
    std::process::exit(75)
}
async fn update_check(app: &Shared) -> Result<Value, String> {
    let repo = app.core.lock().await.settings.repository.clone();
    match updates::check(&repo).await {
        Ok(mut status) => {
            status.pending = app.data.join("pending-update.json").is_file();
            let value = serde_json::to_value(&status).unwrap();
            app.core.lock().await.updates = status;
            Ok(value)
        }
        Err(e) => {
            let mut core = app.core.lock().await;
            core.updates.error = Some(e.clone());
            core.updates.checked_at = Some(crate::now());
            core.updates.available = false;
            Err(e)
        }
    }
}
fn profile(core: &Core, id: &str) -> Result<Profile, String> {
    core.settings
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or("Profil introuvable.".into())
}
fn idle(core: &mut Core, p: &Profile) -> Result<(), String> {
    core.manager.discover(p);
    if core.manager.running(&p.id)
        || core.manager.busy(&p.id)
        || core
            .operations
            .get(&p.id)
            .and_then(|v| v["running"].as_bool())
            .unwrap_or(false)
    {
        return Err("Arrêter le serveur et attendre la fin des opérations.".into());
    }
    Ok(())
}
async fn dispatch(app: Shared, input: Value) -> Result<Value, String> {
    let action = field(&input, "action")?;
    if action == "checkUpdates" {
        return update_check(&app).await;
    }
    if action == "installUpdate" {
        if std::env::var("ARK_PANEL_BOOT").is_err() {
            return Err("Relancer avec Ouvrir ARK Panel.vbs pour activer les mises à jour.".into());
        }
        if app.update_busy.swap(true, Ordering::SeqCst) {
            return Err("Une mise à jour est déjà en cours.".into());
        }
        let repo = {
            let mut core = app.core.lock().await;
            if core.manager.any_busy() || core.operations.values().any(|v| v["running"] == true) {
                app.update_busy.store(false, Ordering::SeqCst);
                return Err(
                    "Attendre la fin des opérations avant la mise à jour du panneau.".into(),
                );
            }
            core.settings.repository.clone()
        };
        let result = updates::prepare(&repo, &app.data).await;
        app.update_busy.store(false, Ordering::SeqCst);
        match result {
            Ok(_) => {
                app.core.lock().await.updates.pending = true;
                Ok(
                    json!({"message":"Version téléchargée et vérifiée. Cliquez sur Redémarrer le panneau."}),
                )
            }
            Err(e) => Err(e),
        }
    } else if action == "restartPanel" {
        let mut core = app.core.lock().await;
        if app.update_busy.load(Ordering::SeqCst)
            || core.manager.any_busy()
            || core.operations.values().any(|v| v["running"] == true)
        {
            return Err("Une opération est en cours.".into());
        }
        if !app.data.join("pending-update.json").is_file() {
            return Err("Aucune mise à jour préparée.".into());
        }
        let app = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            app.shutdown.notify_one();
        });
        Ok(json!({"message":"Redémarrage du panneau ; les processus ARK restent en cours."}))
    } else {
        let mut core = app.core.lock().await;
        if action == "status" {
            let profiles = core.settings.profiles.clone();
            let mut rows = vec![];
            for p in profiles {
                core.manager.discover(&p);
                let running = core.manager.running(&p.id);
                let job = core.manager.job_status(&p.id);
                let operation = core.operations.get(&p.id).cloned();
                rows.push(json!({"profile":p,"running":running,"job":job,"operation":operation}));
            }
            return Ok(
                json!({"version":updates::VERSION,"revision":updates::REVISION,"profiles":rows,"settings":{"repository":core.settings.repository,"branch":core.settings.branch,"steamcmd":core.settings.steamcmd},"updates":core.updates,"updateBusy":app.update_busy.load(Ordering::SeqCst),"dataDirectory":app.data,"lastUpdate":model::load::<Value>(&app.data.join("last-update.json")).unwrap_or(Value::Null)}),
            );
        }
        if app.update_busy.load(Ordering::SeqCst) {
            return Err("Mise à jour du panneau en cours.".into());
        }
        if action == "saveSettings" {
            let repo = field(&input, "repository")?.trim();
            model::validate_repo(repo, "main")?;
            let steamcmd = field(&input, "steamcmd")?.trim();
            let mut next = core.settings.clone();
            next.repository = repo.into();
            next.steamcmd = steamcmd.into();
            model::save(&app.data.join("settings.json"), &next)?;
            core.settings = next;
            core.updates = UpdateStatus {
                current_version: updates::VERSION.into(),
                current_sha: updates::REVISION.into(),
                ..Default::default()
            };
            return Ok(json!({"message":"Paramètres enregistrés."}));
        }
        if action == "saveProfile" {
            let mut p: Profile = serde_json::from_value(input["profile"].clone())
                .map_err(|_| "Profil incomplet ou ports invalides.".to_string())?;
            if p.id.is_empty() {
                p.id = uuid::Uuid::new_v4().to_string();
            } else {
                let old = profile(&core, &p.id)?;
                idle(&mut core, &old)?;
            }
            model::validate_profile(&p, &core.settings.profiles, &app.data)?;
            let mut next = core.settings.clone();
            next.profiles.retain(|o| o.id != p.id);
            next.profiles.push(p.clone());
            model::save(&app.data.join("settings.json"), &next)?;
            core.settings = next;
            return Ok(
                json!({"message":"Profil enregistré. Aucun fichier de serveur existant n’a été modifié.","profile":p}),
            );
        }
        let id = field(&input, "id")?;
        let p = profile(&core, id)?;
        model::validate_profile(&p, &core.settings.profiles, &app.data)?;
        if action == "logs" {
            return Ok(json!({"text":core.manager.logs(&p)?}));
        }
        if action == "readConfig" {
            let file = model::ini_path(&p, field(&input, "file")?)?;
            let bytes = match fs::read(&file) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
                Err(e) => return Err(e.to_string()),
            };
            if bytes.len() > 1024 * 1024 {
                return Err("Fichier trop volumineux pour cet éditeur.".into());
            }
            let text=String::from_utf8(bytes).map_err(|_|"Cet éditeur accepte UTF-8 uniquement. Convertir une copie du fichier avant modification.".to_string())?;
            use sha2::{Digest, Sha256};
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            return Ok(json!({"text":text,"hash":hash}));
        }
        if action == "rcon" || action == "stop" {
            core.manager.discover(&p);
            if core.manager.busy(id)
                || core
                    .operations
                    .get(id)
                    .is_some_and(|v| v["running"] == true)
            {
                return Err("Une opération est en cours sur ce profil.".into());
            }
            let password = model::admin_password(&p)?;
            let text = if action == "stop" {
                "DoExit"
            } else {
                field(&input, "command")?
            };
            if text.is_empty() || text.len() > 4096 || text.contains('\0') {
                return Err("Commande RCON invalide.".into());
            }
            // Keep the operation lock while sending shutdown to prevent a concurrent file operation.
            if action == "stop" {
                crate::rcon::command(p.rcon_port, &password, "SaveWorld").await?;
            }
            let output = crate::rcon::command(p.rcon_port, &password, text).await?;
            return Ok(json!({"message":output}));
        }
        idle(&mut core, &p)?;
        match action {
            "deleteProfile" => {
                let mut next = core.settings.clone();
                next.profiles.retain(|o| o.id != id);
                model::save(&app.data.join("settings.json"), &next)?;
                core.settings = next;
                Ok(json!({"message":"Profil supprimé. Les fichiers du serveur sont conservés."}))
            }
            "start" => {
                core.manager.start(&p)?;
                Ok(
                    json!({"message":"Processus lancé. Attendre la disponibilité du serveur dans ses journaux."}),
                )
            }
            "install" => {
                let steam = core.settings.steamcmd.clone();
                core.manager.install(&p, &steam)?;
                Ok(
                    json!({"message":"SteamCMD lancé. Consultez les journaux pour suivre le téléchargement."}),
                )
            }
            "saveConfig" => {
                let name = field(&input, "file")?;
                let file = model::ini_path(&p, name)?;
                let text = field(&input, "text")?;
                if text.len() > 1024 * 1024 {
                    return Err("Configuration trop volumineuse.".into());
                }
                use sha2::{Digest, Sha256};
                let previous = match fs::read(&file) {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
                    Err(e) => return Err(e.to_string()),
                };
                if format!("{:x}", Sha256::digest(&previous)) != field(&input, "hash")? {
                    return Err(
                        "Le fichier a changé depuis son ouverture. Rechargez-le avant de modifier."
                            .into(),
                    );
                }
                fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
                if file.exists() {
                    fs::copy(
                        &file,
                        file.with_extension(format!("ini.{}.bak", uuid::Uuid::new_v4())),
                    )
                    .map_err(|e| e.to_string())?;
                }
                // save() writes JSON; INI content uses the same atomic replace primitive through save_bytes.
                model::save_bytes(&file, text.as_bytes())?;
                Ok(
                    json!({"message":"Configuration enregistrée ; copie précédente conservée à côté du fichier."}),
                )
            }
            "listBackups" => {
                let dir = app.data.join("backups").join(id);
                let mut items = vec![];
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if entry.path().join("complete.json").is_file() {
                            items.push(entry.file_name().to_string_lossy().to_string());
                        }
                    }
                }
                items.sort();
                items.reverse();
                Ok(json!({"backups":items}))
            }
            "backup" | "restore" => {
                let restore = action == "restore";
                let backup = if restore {
                    Some(field(&input, "backup")?.to_string())
                } else {
                    None
                };
                if let Some(name) = &backup
                    && (name.is_empty()
                        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'))
                {
                    return Err("Nom de sauvegarde invalide.".into());
                }
                core.operations.insert(id.into(),json!({"running":true,"message":if restore{"Restauration en cours"}else{"Sauvegarde en cours"}}));
                let app2 = app.clone();
                let p = p.clone();
                tokio::spawn(async move {
                    let data = app2.data.clone();
                    let id = p.id.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        backup_operation(&p, &data, backup.as_deref())
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r);
                    let mut core = app2.core.lock().await;
                    core.operations.insert(id,json!({"running":false,"success":result.is_ok(),"message":result.unwrap_or_else(|e|e)}));
                });
                Ok(
                    json!({"message":"Opération démarrée. Le serveur doit rester arrêté pendant le traitement."}),
                )
            }
            _ => Err("Action inconnue.".into()),
        }
    }
}
pub fn backup_operation(p: &Profile, data: &Path, restore: Option<&str>) -> Result<String, String> {
    let saved = Path::new(&p.directory).join("ShooterGame/Saved");
    let base = data.join("backups").join(&p.id);
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    if let Some(name) = restore {
        let from = base.join(name);
        let expected: Vec<(String, String)> = model::load(&from.join("complete.json"))?;
        if expected.is_empty() || hash_tree(&from.join("Saved"))? != expected {
            return Err("Sauvegarde absente, vide ou corrompue. Restauration annulée.".into());
        }
        let staging = saved.with_file_name(format!("Saved-restore-{}", uuid::Uuid::new_v4()));
        copy_tree(&from.join("Saved"), &staging)?;
        if hash_tree(&staging)? != expected {
            return Err("Échec de vérification après copie.".into());
        }
        let previous =
            saved.with_file_name(format!("Saved-before-restore-{}", uuid::Uuid::new_v4()));
        let existed = saved.exists();
        if existed {
            fs::rename(&saved, &previous).map_err(|e| e.to_string())?;
        }
        if let Err(e) = fs::rename(&staging, &saved) {
            if existed {
                let _ = fs::rename(&previous, &saved);
            }
            return Err(e.to_string());
        }
        Ok("Sauvegarde restaurée et vérifiée. L’ancien dossier Saved est conservé à côté du nouveau.".into())
    } else {
        let target = base.join(format!("{}-{}", crate::now(), uuid::Uuid::new_v4()));
        copy_tree(&saved, &target.join("Saved"))?;
        let hashes = hash_tree(&saved)?;
        if hashes.is_empty() || hash_tree(&target.join("Saved"))? != hashes {
            return Err(
                "Les fichiers ont changé pendant la copie ou la sauvegarde est vide.".into(),
            );
        }
        model::save(&target.join("complete.json"), &hashes)?;
        Ok(
            "Sauvegarde vérifiée. Les archives restent conservées jusqu’à suppression manuelle."
                .into(),
        )
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    #[tokio::test]
    async fn api_requires_key_and_rejects_cross_site_requests() {
        let data = std::env::temp_dir().join(format!("ark-http-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&data).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Arc::new(App {
            core: Mutex::new(Core {
                settings: Settings::default(),
                manager: Manager::new(data.clone()).unwrap(),
                operations: HashMap::new(),
                updates: UpdateStatus::default(),
            }),
            data: data.clone(),
            token: "test-key-only-not-real".into(),
            host: address.to_string(),
            shutdown: Notify::new(),
            update_busy: AtomicBool::new(false),
        });
        let service =
            tokio::spawn(async move { axum::serve(listener, router(app)).await.unwrap() });
        let client = reqwest::Client::new();
        let url = format!("http://{address}/api");
        let anonymous = client
            .post(&url)
            .json(&json!({"action":"status"}))
            .send()
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
        let cross_site = client
            .post(&url)
            .bearer_auth("test-key-only-not-real")
            .header("Origin", "https://evil.example")
            .json(&json!({"action":"status"}))
            .send()
            .await
            .unwrap();
        assert_eq!(cross_site.status(), StatusCode::UNAUTHORIZED);
        let rebinding = client
            .post(&url)
            .bearer_auth("test-key-only-not-real")
            .header("Host", "evil.example")
            .json(&json!({"action":"status"}))
            .send()
            .await
            .unwrap();
        assert_eq!(rebinding.status(), StatusCode::UNAUTHORIZED);
        let good = client
            .post(&url)
            .bearer_auth("test-key-only-not-real")
            .json(&json!({"action":"status"}))
            .send()
            .await
            .unwrap();
        assert_eq!(good.status(), StatusCode::OK);
        let status: Value = good.json().await.unwrap();
        assert_eq!(status["profiles"], json!([]));
        service.abort();
        let _ = fs::remove_dir_all(data);
    }
}
