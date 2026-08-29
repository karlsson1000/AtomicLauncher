use discord_rich_presence::{
    activity::{Activity, Assets},
    DiscordIpc, DiscordIpcClient,
};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref DISCORD_CLIENT: Mutex<Option<DiscordIpcClient>> = Mutex::new(None);
}

const DISCORD_APP_ID: &str = "";
const LARGE_IMAGE_KEY: &str = "octane";

fn app_id() -> Option<String> {
    let id = if DISCORD_APP_ID.is_empty() {
        std::env::var("DISCORD_APP_ID").unwrap_or_default()
    } else {
        DISCORD_APP_ID.to_string()
    };
    let id = id.trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}

fn is_enabled() -> bool {
    crate::services::settings::SettingsManager::load()
        .map(|settings| settings.discord_rpc)
        .unwrap_or(true)
}

pub fn refresh() {
    if is_enabled() {
        let running_instance = crate::commands::instances::RUNNING_PROCESSES
            .lock()
            .ok()
            .and_then(|processes| processes.keys().next().cloned());

        match running_instance {
            Some(instance_name) => set_activity(&format!("Playing {}", instance_name)),
            None => set_activity("In Launcher"),
        }
    } else {
        with_client(|client| client.clear_activity().is_ok());
    }
}

pub fn set_playing(instance_name: &str) {
    if is_enabled() {
        set_activity(&format!("Playing {}", instance_name));
    }
}

pub fn set_in_launcher() {
    if is_enabled() {
        set_activity("In Launcher");
    }
}

fn set_activity(details: &str) {
    with_client(|client| {
        let activity = Activity::new().details(details).assets(
            Assets::new()
                .large_image(LARGE_IMAGE_KEY)
                .large_text("Octane Launcher"),
        );
        client.set_activity(activity).is_ok()
    });
}

fn with_client(send: impl Fn(&mut DiscordIpcClient) -> bool) {
    let Some(app_id) = app_id() else { return };

    let mut guard = match DISCORD_CLIENT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        let mut client = DiscordIpcClient::new(&app_id);
        if client.connect().is_ok() {
            *guard = Some(client);
        }
    }

    let Some(client) = guard.as_mut() else { return };

    if !send(client) {
        if client.reconnect().is_ok() {
            let _ = send(client);
        } else {
            *guard = None;
        }
    }
}