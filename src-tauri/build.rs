fn main() {
    dotenvy::dotenv().ok();
    println!("cargo:rerun-if-changed=../.env");

    for var in &["MICROSOFT_CLIENT_ID", "DATABASE_URL", "DATABASE_ANON_KEY", "CURSEFORGE_API_KEY"] {
        if let Ok(val) = std::env::var(var) {
            println!("cargo:rustc-env={}={}", var, val);
        }
    }

    let commit = resolve_commit_hash();
    println!("cargo:rustc-env=OCTANE_COMMIT_HASH={}", commit);
    println!("cargo:rerun-if-changed=../.git/HEAD");
    if let Ok(head) = std::fs::read_to_string("../.git/HEAD") {
        if let Some(reference) = head.trim().strip_prefix("ref:") {
            println!("cargo:rerun-if-changed=../.git/{}", reference.trim());
        }
    }

    tauri_build::build()
}

fn resolve_commit_hash() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}
