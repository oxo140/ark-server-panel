use serde_json::{Value, json};
pub fn progress(log: &str, job: &Value) -> Value {
    if job["running"] != true {
        let message = if job["success"] == true && job["publicBuild"]["buildId"].is_string() {
            json!(format!(
                "Version ASA vérifiée : build public {}.",
                job["publicBuild"]["buildId"].as_str().unwrap()
            ))
        } else {
            job["message"].clone()
        };
        return json!({"running":false,"percent":if job["success"]==true{Some(100.)}else{None},"message":message,"success":job["success"]});
    }
    let lower = log.to_lowercase();
    let message = if lower.contains("update state") {
        "Installation / validation des fichiers ASA…"
    } else if lower.contains("waiting for user info") || log.contains("\"2430930\"") {
        "Lecture des informations ASA…"
    } else if lower.contains("connecting anonymously") {
        "Connexion anonyme à Steam…"
    } else {
        "Démarrage et vérification de SteamCMD…"
    };
    let percent = log
        .lines()
        .rev()
        .find(|s| s.contains("Update state") && s.contains("progress:"))
        .and_then(|s| s.split("progress:").nth(1))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
        .filter(|n| job["kind"] != "check" && n.is_finite() && *n >= 0. && *n <= 100.);
    json!({"running":true,"percent":percent,"message":message,"success":null})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_uses_only_real_steam_download_values() {
        let j = json!({"running":true});
        assert!(progress("[ 0%] Checking for updates", &j)["percent"].is_null());
        assert!(
            progress(
                "Update state (0x61) downloading, progress: 100.00 (2 / 2)",
                &json!({"running":true,"kind":"check"})
            )["percent"]
                .is_null()
        );
        assert_eq!(
            progress(
                "Update state (0x61) downloading, progress: 42.50 (1 / 2)",
                &j
            )["percent"],
            42.5
        );
        assert_eq!(
            progress("", &json!({"running":false,"success":true,"message":"OK"}))["percent"],
            100.
        );
    }
}
