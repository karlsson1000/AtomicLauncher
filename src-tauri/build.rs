fn main() {
    dotenvy::dotenv().ok();
    println!("cargo:rerun-if-changed=../.env");

    for var in &["MICROSOFT_CLIENT_ID", "DATABASE_URL", "DATABASE_ANON_KEY", "CURSEFORGE_API_KEY"] {
        if let Ok(val) = std::env::var(var) {
            println!("cargo:rustc-env={}={}", var, val);
        }
    }

    tauri_build::build()
}
