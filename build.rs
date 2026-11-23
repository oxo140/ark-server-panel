fn main() {
    let revision = std::env::var("ARK_BUILD_SHA")
        .ok()
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "development".into());
    println!("cargo:rustc-env=ARK_BUILD_SHA={revision}");
    println!("cargo:rerun-if-env-changed=ARK_BUILD_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/main");
    println!("cargo:rerun-if-changed=web");
}
