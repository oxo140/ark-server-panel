mod catalog;
mod config;
mod connections;
mod console;
mod manager;
mod metrics;
mod model;
mod progress;
mod rcon;
mod server;
mod startup;
mod steamcmd;
mod supervisor;
mod tray;
mod updates;
mod versions;

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--console-job") {
        let result = args
            .get(index + 1)
            .ok_or("Fichier de tâche manquant".to_string())
            .and_then(|p| console::execute(std::path::Path::new(p)));
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!(
            "ark-server-panel {} {}",
            updates::VERSION,
            updates::REVISION
        );
        return;
    }
    let result = if args.iter().any(|a| a == "--serve") {
        server::serve().await
    } else {
        supervisor::supervise().await
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
