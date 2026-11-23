use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

pub fn packet(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(body.len() + 14);
    bytes.extend_from_slice(&((body.len() + 10) as i32).to_le_bytes());
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(body.as_bytes());
    bytes.extend_from_slice(&[0, 0]);
    bytes
}
async fn receive(stream: &mut TcpStream) -> Result<(i32, i32, String), String> {
    let size = stream
        .read_i32_le()
        .await
        .map_err(|_| "Connexion RCON fermée.".to_string())?;
    if !(10..=1048576).contains(&size) {
        return Err("Paquet RCON invalide.".into());
    }
    let mut body = vec![0; size as usize];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| e.to_string())?;
    let id = i32::from_le_bytes(body[0..4].try_into().unwrap());
    let kind = i32::from_le_bytes(body[4..8].try_into().unwrap());
    if body[body.len() - 2..] != [0, 0] {
        return Err("Terminaison RCON invalide.".into());
    }
    Ok((
        id,
        kind,
        String::from_utf8_lossy(&body[8..body.len() - 2]).into(),
    ))
}
pub async fn command(port: u16, password: &str, text: &str) -> Result<String, String> {
    timeout(Duration::from_secs(10), async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.map_err(|_| {
            "RCON inaccessible. Vérifier le port, le serveur et RCONEnabled.".to_string()
        })?;
        stream
            .write_all(&packet(1, 3, password))
            .await
            .map_err(|e| e.to_string())?;
        loop {
            let (id, kind, _) = receive(&mut stream).await?;
            if id == -1 {
                return Err("Mot de passe RCON refusé.".into());
            }
            if id == 1 && kind == 2 {
                break;
            }
        }
        stream
            .write_all(&packet(2, 2, text))
            .await
            .map_err(|e| e.to_string())?;
        let mut result = String::new();
        loop {
            let response = timeout(
                Duration::from_millis(if result.is_empty() { 4000 } else { 400 }),
                receive(&mut stream),
            )
            .await;
            match response {
                Ok(Ok((2, _, body))) => {
                    result.push_str(&body);
                    if result.len() > 1048576 {
                        return Err("Réponse RCON trop longue.".into());
                    }
                    if body.is_empty() {
                        break;
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(e)) if result.is_empty() => return Err(e),
                _ => break,
            }
        }
        if result.is_empty() {
            Ok("Commande envoyée ; aucune réponse textuelle.".into())
        } else {
            Ok(result)
        }
    })
    .await
    .map_err(|_| "Délai RCON dépassé.".to_string())?
}
