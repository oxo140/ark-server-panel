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
    steam_install: Value,
    health: HashMap<String, Value>,
}
pub struct App {
    core: Mutex<Core>,
    data: PathBuf,
    token: String,
    host: String,
    shutdown: Notify,
    update_busy: AtomicBool,
    steam_busy: AtomicBool,
    metrics: crate::metrics::Metrics,
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
        .route("/tailscale-qr.svg",get(||async{secure(crate::tailscale::download_qr(),Some("image/svg+xml; charset=utf-8"))}))
        .route("/",get(||async{secure(Html(include_str!("../web/index.html")),None)}))
        .route("/app.js",get(||async{secure(include_str!("../web/app.js"),Some("application/javascript; charset=utf-8"))}))
        .route("/history.js",get(||async{secure(include_str!("../web/history.js"),Some("application/javascript; charset=utf-8"))}))
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
            steam_install: Value::Null,
            health: HashMap::new(),
        }),
        data: data.clone(),
        token,
        host: format!("{address}:{port}"),
        shutdown: Notify::new(),
        update_busy: AtomicBool::new(false),
        steam_busy: AtomicBool::new(false),
        metrics: crate::metrics::start(data),
    });
    let background = app.clone();
    let health_app = app.clone();
    tokio::spawn(async move {
        loop {
            refresh_health(&health_app).await;
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
    let scheduled = app.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = schedule_tick(&scheduled).await {
                eprintln!("Planification : {e}");
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            let _ = update_check(&background).await;
            tokio::time::sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
        }
    });
    let shutdown = app.clone();
    if std::env::var("ARK_PANEL_NO_TRAY").is_err() {
        let tray_app = app.clone();
        let runtime = tokio::runtime::Handle::current();
        crate::tray::start(
            format!("http://{}/#{}", app.host, app.token),
            Arc::new(move || {
                let app = tray_app.clone();
                runtime.spawn(async move {
                    if let Err(e) = dispatch(app, json!({"action":"quitPanel"})).await {
                        eprintln!("Fermeture refusée : {e}");
                    }
                });
            }),
        );
    }
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
    if action == "tailscaleStatus" {
        let addresses: Vec<_> = crate::connections::candidates()
            .into_iter()
            .filter(|a| a["rank"] == 0)
            .collect();
        return Ok(
            json!({"detected":!addresses.is_empty(),"addresses":addresses,"downloadUrl":crate::tailscale::DOWNLOAD_URL}),
        );
    }
    if action == "startupStatus" {
        let mut value = crate::startup::status();
        value["trayActive"] = json!(crate::tray::active());
        return Ok(value);
    }
    if action == "setStartup" {
        return crate::startup::configure(
            &app.data,
            input["enabled"]
                .as_bool()
                .ok_or("Choix de démarrage requis")?,
        );
    }
    if action == "quitPanel" {
        let mut core = app.core.lock().await;
        if app.update_busy.load(Ordering::SeqCst)
            || app.steam_busy.load(Ordering::SeqCst)
            || core.manager.any_busy()
            || core.operations.values().any(|v| v["running"] == true)
        {
            return Err(
                "Une opération est en cours. Attendez sa fin pour quitter le panneau.".into(),
            );
        }
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            crate::tray::stop();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            std::process::exit(0);
        });
        return Ok(json!({"message":"Fermeture du panneau. Les serveurs ARK restent actifs."}));
    }
    if action == "metrics" {
        return Ok(app
            .metrics
            .read()
            .map_err(|_| "Mesures indisponibles")?
            .clone());
    }
    if action == "browseDirectory" {
        let initial = input["initial"].as_str().unwrap_or("").to_string();
        let selected =
            tokio::task::spawn_blocking(move || crate::steamcmd::pick_directory(&initial))
                .await
                .map_err(|e| e.to_string())??;
        return Ok(json!({"directory":selected,"cancelled":selected.is_none()}));
    }
    if action == "catalog" {
        return Ok(json!({"maps":crate::catalog::MAPS,"mapCatalogDate":"2026-11-23"}));
    }
    if action == "steamcmdStatus" {
        let mut core = app.core.lock().await;
        let configured = input["path"].as_str().unwrap_or(&core.settings.steamcmd);
        let mut result = crate::steamcmd::status(configured, &app.data);
        let init_job = core.manager.job_status("_steamcmd");
        result["installing"] =
            json!(app.steam_busy.load(Ordering::SeqCst) || init_job["running"] == true);
        result["installation"] = if (core.steam_install["phase"] == "init"
            || core.steam_install.is_null())
            && init_job.is_object()
            && !app.steam_busy.load(Ordering::SeqCst)
        {
            init_job
        } else {
            core.steam_install.clone()
        };
        result["logs"] = json!(crate::console::tail(
            &crate::manager::steam_log_path(&app.data, "_steamcmd"),
            64 * 1024
        ));
        result["progress"] = crate::progress::progress(
            result["logs"].as_str().unwrap_or(""),
            &result["installation"],
        );
        return Ok(result);
    }
    if action == "installSteamcmd" {
        if app.update_busy.load(Ordering::SeqCst) {
            return Err("Attendre la fin de la mise à jour du panneau.".into());
        }
        let directory = if let Some(dir) = input["directory"].as_str() {
            PathBuf::from(dir)
        } else {
            crate::steamcmd::managed_path(&app.data)
                .parent()
                .unwrap()
                .to_path_buf()
        };
        crate::steamcmd::validate_directory(&directory)?;
        let app_data = app.data.canonicalize().unwrap_or(app.data.clone());
        let chosen = directory.canonicalize().unwrap_or(directory.clone());
        if chosen == app_data || app_data.starts_with(&chosen) {
            return Err("Choisissez un sous-dossier dédié à SteamCMD, distinct de la racine des données du panneau.".into());
        }
        let visible = false;
        if app.steam_busy.swap(true, Ordering::SeqCst) {
            return Err("SteamCMD est déjà en cours de téléchargement.".into());
        }
        {
            let mut core = app.core.lock().await;
            if core.manager.any_busy() {
                app.steam_busy.store(false, Ordering::SeqCst);
                return Err(
                    "Attendre la fin des installations de serveurs avant de modifier SteamCMD."
                        .into(),
                );
            }
            let normalized = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
            let chosen = PathBuf::from(normalized(&directory));
            if core.settings.profiles.iter().any(|p| {
                let server = PathBuf::from(normalized(Path::new(&p.directory)));
                chosen.starts_with(&server) || server.starts_with(&chosen)
            }) {
                app.steam_busy.store(false, Ordering::SeqCst);
                return Err(
                    "Le dossier SteamCMD doit être distinct des dossiers de serveurs.".into(),
                );
            }
            core.steam_install = json!({"running":true,"phase":"download","message":"Téléchargement de SteamCMD depuis Valve…"});
        }
        let background = app.clone();
        tokio::spawn(async move {
            let result = crate::steamcmd::download_to(&directory).await;
            let mut core = background.core.lock().await;
            let result = result.and_then(|path| {
                let mut next = core.settings.clone();
                next.steamcmd = path.to_string_lossy().into();
                model::save(&background.data.join("settings.json"), &next)?;
                core.settings = next;
                core.manager.steam_job(
                    "_steamcmd",
                    "init",
                    &path.to_string_lossy(),
                    None,
                    visible,
                )?;
                Ok(())
            });
            core.steam_install = json!({"running":result.is_ok(),"phase":if result.is_ok(){"init"}else{"failed"},"success":if result.is_ok(){Value::Null}else{json!(false)},"message":result.map(|_|"SteamCMD téléchargé. Initialisation en cours : consultez le terminal.".to_string()).unwrap_or_else(|e|e)});
            background.steam_busy.store(false, Ordering::SeqCst);
        });
        return Ok(
            json!({"message":"Téléchargement de SteamCMD démarré. Aucun serveur ARK ne sera installé sans votre confirmation."}),
        );
    }
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
            if app.steam_busy.load(Ordering::SeqCst)
                || core.manager.any_busy()
                || core.operations.values().any(|v| v["running"] == true)
            {
                app.update_busy.store(false, Ordering::SeqCst);
                return Err(
                    "Attendre la fin des opérations avant la mise à jour du panneau.".into(),
                );
            }
            core.settings.repository.clone()
        };
        let result = updates::prepare(&repo, &app.data).await;
        match result {
            Ok(_) => {
                app.core.lock().await.updates.pending = true;
                // Keep mutations locked until the supervised restart. The response is
                // delivered before shutting down HTTP; ARK processes are left running.
                app.update_busy.store(true, Ordering::SeqCst);
                let restarting = app.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    restarting.shutdown.notify_one();
                });
                Ok(
                    json!({"message":"Version téléchargée et vérifiée. Redémarrage automatique du panneau ; les serveurs ARK restent actifs.","restarting":true}),
                )
            }
            Err(e) => {
                app.update_busy.store(false, Ordering::SeqCst);
                Err(e)
            }
        }
    } else if action == "restartPanel" {
        let mut core = app.core.lock().await;
        if app.update_busy.load(Ordering::SeqCst)
            || app.steam_busy.load(Ordering::SeqCst)
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
            let connections = crate::connections::load(&app.data)?;
            let candidates = crate::connections::candidates();
            let profiles = core.settings.profiles.clone();
            let mut rows = vec![];
            for p in profiles {
                core.manager.discover(&p);
                let running = core.manager.running(&p.id);
                let resources = core.manager.identities.get(&p.id).and_then(|identity| {
                    let metrics = app.metrics.read().ok()?;
                    let entry = &metrics["processes"][identity.pid.to_string()];
                    (running && entry["startTime"].as_u64() == Some(identity.start))
                        .then(|| entry.clone())
                });
                let job = core.manager.job_status(&p.id);
                let progress = crate::progress::progress(
                    &if job["running"] == true {
                        crate::console::tail(
                            &crate::manager::steam_log_path(&app.data, &p.id),
                            64 * 1024,
                        )
                    } else {
                        String::new()
                    },
                    &job,
                );
                let connection = crate::connections::effective(
                    connections.get(&p.id).cloned().unwrap_or_default(),
                    &candidates,
                );
                let version = crate::versions::view(&app.data, &p, &job);
                let operation = core.operations.get(&p.id).cloned();
                let health = core
                    .health
                    .get(&p.id)
                    .filter(|h| {
                        core.manager
                            .identities
                            .get(&p.id)
                            .is_some_and(|i| h["pid"] == i.pid && h["started"] == i.start)
                    })
                    .cloned()
                    .unwrap_or(Value::Null);
                rows.push(json!({"profile":p,"running":running,"resources":resources,"health":health,"job":job,"progress":progress,"connection":connection,"operation":operation,"serverVersion":version,"executablePresent":crate::manager::executable(&p).is_file()}));
            }
            return Ok(
                json!({"version":updates::VERSION,"revision":updates::REVISION,"profiles":rows,"settings":{"repository":core.settings.repository,"branch":core.settings.branch,"steamcmd":core.settings.steamcmd},"updates":core.updates,"updateBusy":app.update_busy.load(Ordering::SeqCst),"steamInstalling":app.steam_busy.load(Ordering::SeqCst),"dataDirectory":app.data,"lastUpdate":model::load::<Value>(&app.data.join("last-update.json")).unwrap_or(Value::Null)}),
            );
        }
        if app.update_busy.load(Ordering::SeqCst) {
            return Err("Mise à jour du panneau en cours.".into());
        }
        if action == "saveSettings" {
            if app.steam_busy.load(Ordering::SeqCst)
                || core.manager.any_busy()
                || core.operations.values().any(|v| v["running"] == true)
            {
                return Err(
                    "Attendre la fin de l’installation SteamCMD avant de modifier son chemin."
                        .into(),
                );
            }
            let repo = field(&input, "repository")?.trim();
            model::validate_repo(repo, "main")?;
            let steamcmd = field(&input, "steamcmd")?.trim();
            if !steamcmd.is_empty() && !crate::steamcmd::valid_executable(Path::new(steamcmd)) {
                return Err("SteamCMD : fichier introuvable ou exécutable Windows invalide. Vérifiez le chemin ou utilisez Télécharger SteamCMD.".into());
            }
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
        if action == "connectionInfo" || action == "setConnection" {
            let mut connections = crate::connections::load(&app.data)?;
            let candidates = crate::connections::candidates();
            let mut entry = crate::connections::effective(
                connections.get(id).cloned().unwrap_or_default(),
                &candidates,
            );
            if action == "setConnection" {
                if let Some(host) = input["host"].as_str() {
                    entry.host = crate::connections::connection_host(host, p.game_port)?;
                    entry.automatic = false;
                }
                if let Some(favorite) = input["favorite"].as_bool() {
                    entry.favorite = favorite;
                }
                connections.insert(id.into(), entry.clone());
                model::save(&app.data.join("connections.json"), &connections)?;
            }
            let address = if entry.host.is_empty() {
                String::new()
            } else {
                crate::connections::endpoint(&entry.host, p.game_port)
            };
            return Ok(
                json!({"connection":entry,"address":address,"gamePort":p.game_port,"candidates":if action=="connectionInfo"{candidates}else{vec![]},"message":"Adresse et favori enregistrés."}),
            );
        }
        model::validate_profile(&p, &core.settings.profiles, &app.data)?;
        if action == "readSchedule" || action == "saveSchedule" {
            let path = app.data.join(format!("schedule-{id}.json"));
            let previous: crate::schedule::Schedule = model::load(&path)?;
            if action == "readSchedule" {
                return Ok(serde_json::to_value(previous).unwrap());
            }
            if core
                .operations
                .get(id)
                .is_some_and(|v| v["running"] == true)
            {
                return Err(
                    "Attendez la fin de la maintenance avant de modifier sa programmation.".into(),
                );
            }
            let mut next: crate::schedule::Schedule =
                serde_json::from_value(input["schedule"].clone()).map_err(|e| e.to_string())?;
            next.validate()?;
            next.last_backup = if !previous.backup_enabled && next.backup_enabled {
                crate::now()
            } else {
                previous.last_backup
            };
            next.last_restart_day = previous.last_restart_day;
            next.last_update_day = previous.last_update_day;
            next.last_result = previous.last_result;
            model::save(&path, &next)?;
            return Ok(
                json!({"message":"Planification enregistrée. Le panneau doit rester ouvert dans la zone de notification."}),
            );
        }
        if action == "serverStats" {
            core.manager.discover(&p);
            let mut system = sysinfo::System::new();
            if let Some(identity) = core.manager.identities.get(id) {
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(identity.pid)]),
                    true,
                    sysinfo::ProcessRefreshKind::nothing().with_memory(),
                );
            }
            let process = core.manager.identities.get(id).and_then(|i| {
                system
                    .process(sysinfo::Pid::from_u32(i.pid))
                    .filter(|p| p.start_time() == i.start)
            });
            let running = core.manager.running(id);
            drop(core);
            let health = assess_health(
                &app,
                &p,
                running,
                process.map(|p| p.pid().as_u32()).unwrap_or(0),
                process.map(|p| p.start_time()).unwrap_or(0),
            )
            .await;
            app.core
                .lock()
                .await
                .health
                .insert(p.id.clone(), health.clone());
            return Ok(
                json!({"running":running,"ready":health["ready"],"health":health,"probeError":health["error"],"players":health["players"],"pid":process.map(|p|p.pid().as_u32()),"memoryBytes":process.map(|p|p.memory()),"uptimeSeconds":process.map(|p|p.run_time()),"sampledAt":crate::now()}),
            );
        }

        if action == "exportTuning" {
            let tuning = crate::config::read(&p)?;
            let values: serde_json::Map<String, Value> = tuning["fields"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["configured"] == true && f["warning"].is_null())
                .map(|f| (f["key"].as_str().unwrap().to_string(), f["value"].clone()))
                .collect();
            return Ok(json!({"format":"ark-panel-tuning-v1","values":values}));
        }
        if action == "logs" {
            let text = core.manager.logs(&p)?;
            let job = core.manager.job_status(&p.id);
            let steam =
                crate::console::tail(&crate::manager::steam_log_path(&app.data, &p.id), 64 * 1024);
            return Ok(json!({"text":text,"progress":crate::progress::progress(&steam,&job)}));
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
        if action == "readTuning" {
            return crate::config::read(&p);
        }
        if action == "previewTuning" {
            return crate::simulation::preview(&p, &input["values"], &input["reference"]);
        }
        if action == "checkServerVersion" {
            if app.steam_busy.load(Ordering::SeqCst)
                || core.manager.busy(id)
                || core
                    .operations
                    .get(id)
                    .is_some_and(|v| v["running"] == true)
            {
                return Err("Attendre la fin de l’opération SteamCMD en cours.".into());
            }
            let detected = crate::steamcmd::status(&core.settings.steamcmd, &app.data);
            let steam = detected["path"]
                .as_str()
                .ok_or("Installez SteamCMD avant de vérifier la version.")?;
            core.manager.steam_job(id, "check", steam, None, false)?;
            return Ok(
                json!({"message":"Vérification du dernier build public Steam ASA. Les fichiers du serveur ne seront pas mis à jour."}),
            );
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
        if action != "listBackups" {
            idle(&mut core, &p)?;
        }
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
                if app.steam_busy.load(Ordering::SeqCst)
                    || core
                        .operations
                        .get(id)
                        .is_some_and(|v| v["running"] == true)
                {
                    return Err("Attendre la fin du téléchargement SteamCMD.".into());
                }
                let detected = crate::steamcmd::status(&core.settings.steamcmd, &app.data);
                let steam=detected["path"].as_str().ok_or("SteamCMD introuvable : télécharger SteamCMD ou sélectionner un chemin valide dans les paramètres.")?.to_string();
                if core.settings.steamcmd != steam {
                    let mut next = core.settings.clone();
                    next.steamcmd = steam.clone();
                    model::save(&app.data.join("settings.json"), &next)?;
                    core.settings = next;
                }
                core.manager.install(&p, &steam, false)?;
                Ok(
                    json!({"message":"SteamCMD lancé. Consultez les journaux pour suivre le téléchargement."}),
                )
            }
            "confirmServerUpdate" => {
                if app.steam_busy.load(Ordering::SeqCst)
                    || core.manager.busy(id)
                    || core
                        .operations
                        .get(id)
                        .is_some_and(|v| v["running"] == true)
                {
                    return Err("Une opération SteamCMD est déjà en cours.".into());
                }
                let checked = model::load::<Value>(&crate::versions::cache_path(&app.data, id))?;
                if checked["buildId"].as_str() != input["checkedBuild"].as_str()
                    || checked["buildId"].is_null()
                    || crate::now().saturating_sub(checked["checkedAt"].as_u64().unwrap_or(0))
                        > 1800
                {
                    return Err("Vérifiez de nouveau la version du serveur avant de confirmer la mise à jour.".into());
                }
                let detected = crate::steamcmd::status(&core.settings.steamcmd, &app.data);
                let steam = detected["path"].as_str().ok_or("SteamCMD introuvable.")?;
                fs::create_dir_all(&p.directory).map_err(|e| e.to_string())?;
                core.manager.steam_job(
                    id,
                    "update",
                    steam,
                    Some(Path::new(&p.directory)),
                    false,
                )?;
                Ok(
                    json!({"message":"Mise à jour du serveur confirmée et démarrée. Consultez le terminal."}),
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
            "saveTuning" => {
                crate::config::save(&p, &input["hashes"], &input["changes"])?;
                Ok(
                    json!({"message":"Réglages enregistrés. Les autres paramètres sont conservés et les fichiers précédents ont été sauvegardés."}),
                )
            }
            "importTuning" => {
                if input["preset"]["format"] != "ark-panel-tuning-v1" {
                    return Err("Format de profil incompatible.".into());
                }
                crate::config::save(&p, &input["hashes"], &input["preset"]["values"])?;
                Ok(
                    json!({"message":"Profil de réglages importé. Les adresses, mots de passe et ports restent propres au serveur."}),
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
async fn assess_health(app: &Shared, p: &Profile, running: bool, pid: u32, started: u64) -> Value {
    let probe = if running {
        match model::admin_password(p) {
            Ok(pass) => crate::rcon::command(p.rcon_port, &pass, "ListPlayers").await,
            Err(e) => Err(e),
        }
    } else {
        Err("Serveur arrêté".into())
    };
    let previous = app
        .core
        .lock()
        .await
        .health
        .get(&p.id)
        .cloned()
        .unwrap_or(Value::Null);
    let advertised = running
        && (crate::readiness::advertised(p, started)
            || (previous["pid"] == pid
                && previous["started"] == started
                && previous["advertised"] == true));
    let game_port = running && crate::readiness::game_port_bound(pid, p.game_port);
    let ready = running && game_port && (advertised || probe.is_ok());
    let state = if !running {
        "offline"
    } else if ready {
        "ready"
    } else if crate::now().saturating_sub(started) < 900 {
        "starting"
    } else {
        "unreachable"
    };
    json!({"pid":pid,"started":started,"state":state,"ready":ready,"checkedAt":crate::now(),"rconReady":probe.is_ok(),"advertised":advertised,"gamePortBound":game_port,"remoteVerified":false,"error":probe.as_ref().err(),"players":probe.as_ref().ok(),"message":if ready {"Serveur opérationnel localement : port UDP détenu par ARK et disponibilité confirmée. L’accès depuis un autre PC Tailscale reste à tester."}else if !running {"Serveur arrêté."}else {"Démarrage ou disponibilité du port de jeu non confirmé."}})
}
async fn refresh_health(app: &Shared) {
    let profiles = app.core.lock().await.settings.profiles.clone();
    for p in profiles {
        let (running, pid, started) = {
            let mut c = app.core.lock().await;
            c.manager.discover(&p);
            let running = c.manager.running(&p.id);
            let identity = c.manager.identities.get(&p.id);
            (
                running,
                identity.map(|i| i.pid).unwrap_or(0),
                identity.map(|i| i.start).unwrap_or(0),
            )
        };
        let health = assess_health(app, &p, running, pid, started).await;
        app.core.lock().await.health.insert(p.id, health);
    }
}
async fn wait_steam(app: &Shared, id: &str) -> Result<(), String> {
    for _ in 0..7200 {
        let job = app.core.lock().await.manager.job_status(id);
        if job["running"] != true {
            if job["kind"] == "check"
                && job["success"] == true
                && job["publicBuild"]["buildId"].is_string()
            {
                model::save(
                    &crate::versions::cache_path(&app.data, id),
                    &job["publicBuild"],
                )?;
            }
            return if job["success"] == true {
                Ok(())
            } else {
                Err(job["message"].as_str().unwrap_or("Échec SteamCMD").into())
            };
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Err("Délai SteamCMD dépassé ; serveur conservé à l’arrêt.".into())
}
async fn maintenance(
    app: &Shared,
    p: &Profile,
    s: &crate::schedule::Schedule,
    backup: bool,
    restart: bool,
    update: bool,
) -> Result<String, String> {
    let mut changed = false;
    if update {
        {
            let mut c = app.core.lock().await;
            let steam = crate::steamcmd::status(&c.settings.steamcmd, &app.data)["path"]
                .as_str()
                .ok_or("SteamCMD introuvable")?
                .to_string();
            c.manager.steam_job(&p.id, "check", &steam, None, false)?;
        }
        wait_steam(app, &p.id).await?;
        let latest: Value = model::load(&crate::versions::cache_path(&app.data, &p.id))?;
        let installed = crate::versions::installed(Path::new(&p.directory));
        if latest["buildId"].as_str().is_none() {
            return Err("Build public non vérifié.".into());
        }
        changed = latest["buildId"] != installed["buildId"] || installed["complete"] != true;
    }
    if !backup && !restart && !changed {
        return Ok("Vérification automatique : serveur à jour.".into());
    }
    let was_running = app.core.lock().await.manager.running(&p.id);
    if was_running {
        let password = model::admin_password(p)?;
        for remaining in (1..=s.warning_minutes).rev() {
            let message = s.message.replace("{minutes}", &remaining.to_string());
            crate::rcon::command(p.rcon_port, &password, &format!("Broadcast {message}")).await?;
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
        crate::rcon::command(p.rcon_port, &password, "SaveWorld").await?;
        crate::rcon::command(p.rcon_port, &password, "DoExit").await?;
        let mut stopped = false;
        for _ in 0..150 {
            if !app.core.lock().await.manager.running(&p.id) {
                stopped = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        if !stopped {
            return Err("Arrêt non confirmé : maintenance annulée, aucun arrêt forcé.".into());
        }
    }
    // Keep a verified copy before every scheduled update as well as requested backups.
    let result:Result<(),String>=async {
        if backup || changed {
            let p=p.clone();let data=app.data.clone();
            tokio::task::spawn_blocking(move||backup_operation(&p,&data,None)).await.map_err(|e|e.to_string())??;
        }
        if changed {
            {
                let mut c=app.core.lock().await;
                let steam=crate::steamcmd::status(&c.settings.steamcmd,&app.data)["path"].as_str().ok_or("SteamCMD introuvable")?.to_string();
                c.manager.steam_job(&p.id,"update",&steam,Some(Path::new(&p.directory)),false)?;
            }
            wait_steam(app,&p.id).await?;
            let latest:Value=model::load(&crate::versions::cache_path(&app.data,&p.id))?;
            let installed=crate::versions::installed(Path::new(&p.directory));
            if installed["complete"]!=true || installed["buildId"]!=latest["buildId"] {
                return Err("Le build installé ne correspond pas au build vérifié. Serveur conservé à l’arrêt.".into());
            }
        }
        Ok(())
    }.await;
    // A failed update may leave incomplete binaries. Never restart those automatically.
    if was_running && (result.is_ok() || !changed) {
        app.core.lock().await.manager.start(p)?;
    }
    result?;
    Ok("Maintenance terminée : sauvegarde / mise à jour vérifiée, serveur relancé s’il était actif.".into())
}
async fn schedule_tick(app: &Shared) -> Result<(), String> {
    let profiles = app.core.lock().await.settings.profiles.clone();
    for p in profiles {
        let path = app.data.join(format!("schedule-{}.json", p.id));
        let mut s: crate::schedule::Schedule = model::load(&path)?;
        s.validate()?;
        let (day, minute) = crate::schedule::local_clock();
        let (backup, restart, update) = s.due(crate::now(), day, minute);
        if !backup && !restart && !update {
            continue;
        }
        {
            let mut c = app.core.lock().await;
            if app.update_busy.load(Ordering::SeqCst)
                || app.steam_busy.load(Ordering::SeqCst)
                || c.manager.busy(&p.id)
                || c.operations
                    .get(&p.id)
                    .is_some_and(|v| v["running"] == true)
            {
                continue;
            }
            model::validate_profile(&p, &c.settings.profiles, &app.data)?;
            if backup {
                s.last_backup = crate::now();
            }
            if restart {
                s.last_restart_day = day;
            }
            if update {
                s.last_update_day = day;
            }
            s.last_result = "Maintenance en cours".into();
            model::save(&path, &s)?;
            c.operations.insert(p.id.clone(),json!({"running":true,"message":"Maintenance programmée en cours (préavis inclus)"}));
        }
        let result = maintenance(app, &p, &s, backup, restart, update).await;
        s.last_result = format!(
            "{} : {}",
            crate::now(),
            result
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or_else(|e| e.as_str())
        );
        let saved = model::save(&path, &s);
        app.core.lock().await.operations.insert(
            p.id.clone(),
            json!({"running":false,"success":result.is_ok(),"message":s.last_result}),
        );
        saved?;
    }
    Ok(())
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
    fn fixture() -> (Shared, Profile, PathBuf) {
        let root = std::env::temp_dir().join(format!("ark-schedule-test-{}", uuid::Uuid::new_v4()));
        let data = root.join("panel");
        fs::create_dir_all(&data).unwrap();
        let p = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test".into(),
            directory: root.join("server").to_string_lossy().into(),
            map: "TheIsland_WP".into(),
            game_port: 17777,
            rcon_port: 17778,
            mods: String::new(),
        };
        let app = Arc::new(App {
            core: Mutex::new(Core {
                settings: Settings {
                    profiles: vec![p.clone()],
                    ..Default::default()
                },
                manager: Manager::new(data.clone()).unwrap(),
                operations: HashMap::new(),
                updates: UpdateStatus::default(),
                steam_install: Value::Null,
                health: HashMap::new(),
            }),
            data,
            token: "test".into(),
            host: "localhost".into(),
            shutdown: Notify::new(),
            update_busy: AtomicBool::new(false),
            steam_busy: AtomicBool::new(false),
            metrics: crate::metrics::empty(),
        });
        (app, p, root)
    }
    #[tokio::test]
    async fn scheduled_backup_is_verified_and_not_repeated_after_reload() {
        let (app, p, root) = fixture();
        let saved = Path::new(&p.directory).join("ShooterGame/Saved");
        fs::create_dir_all(&saved).unwrap();
        fs::write(saved.join("world.ark"), b"world fixture").unwrap();
        let path = app.data.join(format!("schedule-{}.json", p.id));
        model::save(
            &path,
            &crate::schedule::Schedule {
                backup_enabled: true,
                ..Default::default()
            },
        )
        .unwrap();
        schedule_tick(&app).await.unwrap();
        let backups = app.data.join("backups").join(&p.id);
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 1);
        let archive = fs::read_dir(&backups)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(archive.join("complete.json").is_file());
        assert_eq!(
            fs::read(archive.join("Saved/world.ark")).unwrap(),
            b"world fixture"
        );
        schedule_tick(&app).await.unwrap();
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 1);
        let s: crate::schedule::Schedule = model::load(&path).unwrap();
        assert!(s.last_backup > 0);
        assert!(
            !app.core.lock().await.operations[&p.id]["running"]
                .as_bool()
                .unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn tuning_transfer_excludes_secrets_and_preserves_target_identity() {
        let (app, p, root) = fixture();
        let file = model::ini_path(&p, "GameUserSettings.ini").unwrap();
        model::save_bytes(
            &file,
            b"[ServerSettings]\nServerAdminPassword=secret\nRCONPort=17778\nXPMultiplier=3\n",
        )
        .unwrap();
        let exported = dispatch(app.clone(), json!({"action":"exportTuning","id":p.id}))
            .await
            .unwrap();
        assert_eq!(exported["values"]["XPMultiplier"], 3.0);
        assert!(!exported.to_string().contains("secret"));
        let read = crate::config::read(&p).unwrap();
        let mut preset = exported.clone();
        preset["values"]["XPMultiplier"] = json!(5);
        dispatch(
            app.clone(),
            json!({"action":"importTuning","id":p.id,"preset":preset,"hashes":read["hashes"]}),
        )
        .await
        .unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("XPMultiplier=5"));
        assert!(text.contains("RCONPort=17778"));
        assert!(text.contains("ServerAdminPassword=secret"));
        assert!(
            dispatch(
                app,
                json!({"action":"importTuning","id":p.id,"preset":exported,"hashes":read["hashes"]})
            )
            .await
            .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
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
                steam_install: Value::Null,
                health: HashMap::new(),
            }),
            data: data.clone(),
            token: "test-key-only-not-real".into(),
            host: address.to_string(),
            shutdown: Notify::new(),
            update_busy: AtomicBool::new(false),
            steam_busy: AtomicBool::new(false),
            metrics: crate::metrics::empty(),
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
