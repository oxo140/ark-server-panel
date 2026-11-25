use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default = "automatic")]
    pub automatic: bool,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub favorite: bool,
}
fn automatic() -> bool {
    true
}
impl Default for Entry {
    fn default() -> Self {
        Self {
            host: String::new(),
            favorite: false,
            automatic: true,
        }
    }
}
pub fn effective(mut entry: Entry, candidates: &[Value]) -> Entry {
    if entry.automatic {
        if let Some(tail) = candidates.iter().find(|v| v["rank"] == 0) {
            entry.host = tail["host"].as_str().unwrap_or_default().into();
        } else if entry.host.is_empty() {
            entry.host = candidates
                .first()
                .and_then(|c| c["host"].as_str())
                .unwrap_or_default()
                .into();
        }
    }
    entry
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
    for ip in tailscale_addresses() {
        let host = ip.to_string();
        found.insert(
            host.clone(),
            json!({"host":host,"label":format!("{ip} — Tailscale"),"rank":0}),
        );
    }
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
// Tunnel adapters can be absent from sysinfo's traffic-counter interface list.
#[cfg(windows)]
fn tailscale_addresses() -> Vec<Ipv4Addr> {
    use windows_sys::Win32::{NetworkManagement::IpHelper::*, Networking::WinSock::*};
    let mut found = Vec::new();
    unsafe {
        let mut size = 16384;
        for _ in 0..3 {
            if size > 16 * 1024 * 1024 {
                return found;
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            let result = GetAdaptersAddresses(
                AF_INET as u32,
                GAA_FLAG_INCLUDE_ALL_INTERFACES
                    | GAA_FLAG_SKIP_ANYCAST
                    | GAA_FLAG_SKIP_MULTICAST
                    | GAA_FLAG_SKIP_DNS_SERVER,
                std::ptr::null(),
                first,
                &mut size,
            );
            if result == 111 {
                continue;
            }
            if result != 0 {
                return found;
            }
            let mut adapter = first;
            while !adapter.is_null() {
                let a = &*adapter;
                let mut length = 0;
                if !a.FriendlyName.is_null() {
                    while length < 1024 && *a.FriendlyName.add(length) != 0 {
                        length += 1;
                    }
                }
                let name = if length > 0 {
                    String::from_utf16_lossy(std::slice::from_raw_parts(a.FriendlyName, length))
                } else {
                    String::new()
                };
                if name.to_lowercase().contains("tailscale") && a.OperStatus == 1 {
                    let mut unicast = a.FirstUnicastAddress;
                    while !unicast.is_null() {
                        let address = &(*unicast).Address;
                        if !address.lpSockaddr.is_null()
                            && address.iSockaddrLength as usize
                                >= std::mem::size_of::<SOCKADDR_IN>()
                            && (*address.lpSockaddr).sa_family == AF_INET
                        {
                            let v4 = &*address.lpSockaddr.cast::<SOCKADDR_IN>();
                            let ip = Ipv4Addr::from(v4.sin_addr.S_un.S_addr.to_ne_bytes());
                            if ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]) {
                                found.push(ip);
                            }
                        }
                        unicast = (*unicast).Next;
                    }
                }
                adapter = a.Next;
            }
            break;
        }
    }
    found
}
#[cfg(not(windows))]
fn tailscale_addresses() -> Vec<Ipv4Addr> {
    vec![]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_lan_address_prefers_tailscale_but_manual_choice_is_kept() {
        let entry: Entry =
            serde_json::from_value(json!({"host":"192.168.1.23","favorite":true})).unwrap();
        let rows = vec![
            json!({"host":"100.89.37.18","rank":0}),
            json!({"host":"192.168.1.23","rank":1}),
        ];
        let selected = effective(entry, &rows);
        assert_eq!(endpoint(&selected.host, 7777), "100.89.37.18:7777");
        assert!(selected.favorite);
        let manual = effective(
            Entry {
                host: "192.168.1.23".into(),
                automatic: false,
                favorite: false,
            },
            &rows,
        );
        assert_eq!(manual.host, "192.168.1.23");
    }
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
