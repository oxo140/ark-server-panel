use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug)]
enum Vdf {
    Text(String),
    Object(BTreeMap<String, Vdf>),
}
impl Vdf {
    fn get(&self, path: &[&str]) -> Option<&str> {
        if path.is_empty() {
            if let Self::Text(s) = self {
                Some(s)
            } else {
                None
            }
        } else if let Self::Object(m) = self {
            m.get(path[0])?.get(&path[1..])
        } else {
            None
        }
    }
}
fn tokens(text: &str) -> Result<Vec<String>, String> {
    let mut chars = text.chars().peekable();
    let mut out = vec![];
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' | '}' => out.push(c.to_string()),
            '"' => {
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '"' {
                        closed = true;
                        break;
                    }
                    if c == '\\' {
                        if let Some(n) = chars.next() {
                            if n == '"' || n == '\\' {
                                s.push(n);
                            } else {
                                s.push('\\');
                                s.push(n);
                            }
                        }
                    } else {
                        s.push(c);
                    }
                }
                if !closed {
                    return Err("Texte VDF incomplet.".into());
                }
                out.push(s);
            }
            _ => {
                let mut s = c.to_string();
                while let Some(c) = chars.peek() {
                    if c.is_whitespace() || *c == '{' || *c == '}' {
                        break;
                    }
                    s.push(chars.next().unwrap());
                }
                out.push(s);
            }
        }
    }
    Ok(out)
}
fn object(tokens: &[String], at: &mut usize, depth: usize) -> Result<Vdf, String> {
    if depth > 32 || tokens.get(*at).map(String::as_str) != Some("{") {
        return Err("Structure VDF invalide.".into());
    }
    *at += 1;
    let mut map = BTreeMap::new();
    loop {
        let key = tokens.get(*at).ok_or("Structure VDF incomplète")?.clone();
        *at += 1;
        if key == "}" {
            return Ok(Vdf::Object(map));
        }
        if key == "{" {
            return Err("Clé VDF invalide.".into());
        }
        let value = if tokens.get(*at).map(String::as_str) == Some("{") {
            object(tokens, at, depth + 1)?
        } else {
            let value = tokens.get(*at).ok_or("Valeur VDF absente")?.clone();
            if value == "}" {
                return Err("Valeur VDF absente.".into());
            }
            *at += 1;
            Vdf::Text(value)
        };
        map.insert(key, value);
    }
}
fn root(text: &str, name: &str) -> Result<Vdf, String> {
    let ts = tokens(text)?;
    for i in 0..ts.len().saturating_sub(1) {
        if ts[i] == name && ts[i + 1] == "{" {
            let mut at = i + 1;
            if let Ok(value) = object(&ts, &mut at, 0) {
                return Ok(value);
            }
        }
    }
    Err(format!("Informations Steam absentes : {name}"))
}
fn build(value: Option<&str>) -> Option<String> {
    value
        .filter(|s| {
            !s.is_empty() && s.len() <= 20 && s.chars().all(|c| c.is_ascii_digit()) && *s != "0"
        })
        .map(String::from)
}
pub fn public_build(text: &str) -> Result<Value, String> {
    let data = root(text, "2430930")?;
    let id = build(data.get(&["depots", "branches", "public", "buildid"]))
        .ok_or("Le build public ASA est absent de la réponse SteamCMD.")?;
    let updated = data
        .get(&["depots", "branches", "public", "timeupdated"])
        .and_then(|s| s.parse::<u64>().ok());
    Ok(json!({"buildId":id,"updatedAt":updated,"checkedAt":crate::now()}))
}
pub fn installed(directory: &Path) -> Value {
    for file in [
        directory.join("steamapps/appmanifest_2430930.acf"),
        directory.join("appmanifest_2430930.acf"),
    ] {
        if let Ok(text) = fs::read_to_string(&file)
            && let Ok(data) = root(&text, "AppState")
            && data.get(&["appid"]) == Some("2430930")
        {
            let flags = data
                .get(&["StateFlags"])
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            return json!({"buildId":build(data.get(&["buildid"])),"complete":flags==4,"manifest":file,"lastUpdated":data.get(&["LastUpdated"]).and_then(|s|s.parse::<u64>().ok())});
        }
    }
    json!({"buildId":null,"complete":false,"manifest":null})
}
pub fn cache_path(data: &Path, id: &str) -> PathBuf {
    data.join(format!("server-version-{id}.json"))
}
pub fn view(data: &Path, p: &crate::model::Profile, job: &Value) -> Value {
    let cache = cache_path(data, &p.id);
    if job["kind"] == "check"
        && job["success"] == true
        && job["publicBuild"]["buildId"].is_string()
        && crate::model::load::<Value>(&cache).unwrap_or(Value::Null) != job["publicBuild"]
    {
        let _ = crate::model::save(&cache, &job["publicBuild"]);
    }
    let remote = crate::model::load::<Value>(&cache).unwrap_or(Value::Null);
    let local = installed(Path::new(&p.directory));
    let comparable = local["buildId"].is_string() && remote["buildId"].is_string();
    json!({"installed":local,"latest":remote,"upToDate":comparable&&local["complete"]==true&&local["buildId"]==remote["buildId"],"updateAvailable":comparable&&local["buildId"]!=remote["buildId"],"checked":remote["buildId"].is_string()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_branch_is_selected_instead_of_depot_or_beta_build() {
        let input = r#"Connecting...
"2430930" { "depots" { "100" { "buildid" "1" } "branches" { "beta" { "buildid" "9999" } "public" { "buildid" "12345" "timeupdated" "1700000000" } } } }
Steam>quit"#;
        let result = public_build(input).unwrap();
        assert_eq!(result["buildId"], "12345");
        assert_eq!(result["updatedAt"], 1700000000u64);
        assert!(public_build("Error! No app info").is_err());
    }
}
