mod catalog;
mod config;
mod connections;
mod console;
mod manager;
mod metrics;
mod model;
mod progress;
mod rcon;
mod schedule;
mod server;
mod startup;
mod steamcmd;
mod supervisor;
mod tray;
mod updates;
mod versions;
mod window_guard;

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn main() {
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
    // Console workers only need synchronous I/O; do not create a web runtime for each job.
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(8)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
        .and_then(|runtime| {
            runtime.block_on(async {
                if args.iter().any(|a| a == "--serve") {
                    server::serve().await
                } else {
                    supervisor::supervise().await
                }
            })
        });
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
