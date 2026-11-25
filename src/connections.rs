use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub favorite: bool,
}
pub type Entries = BTreeMap<String, Entry>;
pub fn load(data: &Path) -> Result<Entries, String> {
    crate::model::load(&data.join("connections.json"))
}
pub fn valid_host(input: &str) -> Result<String, String> {
    let host = input.trim().trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_unspecified() || ip.is_multicast() {
            return Err("Adresse de connexion non valide.".into());
        }
        return Ok(ip.to_string());
    }
    if host.is_empty()
        || host.len() > 253
        || host.chars().all(|c| c.is_ascii_digit() || c == '.')
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
    {
        return Err("Saisir une IP ou un nom DNS, sans protocole ni port. Le port de jeu sera ajouté automatiquement.".into());
    }
    Ok(host.to_lowercase())
}
pub fn endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
pub fn candidates() -> Vec<Value> {
    let nets = sysinfo::Networks::new_with_refreshed_list();
    let mut found = BTreeMap::new();
    for (name, net) in &nets {
        for addr in net.ip_networks() {
            if let IpAddr::V4(ip) = addr.addr {
                if ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_link_local()
                    || ip.is_multicast()
                {
                    continue;
                }
                let tail = ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]);
                let virtual_net = ["vmware", "vethernet", "virtualbox"]
                    .iter()
                    .any(|s| name.to_lowercase().contains(s));
                let rank = if tail {
                    0
                } else if virtual_net {
                    3
                } else {
                    1
                };
                let host = ip.to_string();
                found.entry(host.clone()).or_insert(json!({"host":host,"label":format!("{} — {} ({})",ip,if tail{"Tailscale"}else{"Réseau local"},name),"rank":rank}));
            }
        }
    }
    let mut rows: Vec<_> = found.into_values().collect();
    rows.sort_by_key(|r| r["rank"].as_u64().unwrap_or(5));
    rows.push(json!({"host":Ipv4Addr::LOCALHOST.to_string(),"label":"127.0.0.1 — seulement sur la machine du serveur","rank":9}));
    rows
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joining_uses_game_port_and_rejects_command_injection() {
        assert_eq!(
            endpoint(&valid_host("100.101.102.103").unwrap(), 7777),
            "100.101.102.103:7777"
        );
        assert_eq!(endpoint(&valid_host("[::1]").unwrap(), 7787), "[::1]:7787");
        assert!(valid_host("1.2.3.4:27020").is_err());
        assert!(valid_host("1.2.3.4?Password=secret").is_err());
        assert!(valid_host("x\" -ExecCmds=quit").is_err());
        assert!(valid_host("999.999.999.999").is_err());
        assert_eq!(
            valid_host("SERVER.tailnet.ts.net").unwrap(),
            "server.tailnet.ts.net"
        );
    }
}
