use std::sync::OnceLock;
use std::time::Duration;

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .user_agent(concat!(
            "OctaneLauncher/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/karlsson1000/OctaneLauncher)"
        ))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client")
}

pub fn get_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(build_client).clone()
}
