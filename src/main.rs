mod catalog;
mod config;
mod manager;
mod model;
mod rcon;
mod server;
mod steamcmd;
mod supervisor;
mod updates;

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
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
