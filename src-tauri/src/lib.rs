mod auth;
mod commands;
mod services;
mod utils;
mod models;

use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;
use services::accounts::AccountManager;
use services::friends::FriendsService;
use models::{AppConfig, FriendStatus};
use tauri_plugin_store::StoreExt;
use std::sync::Arc;

use commands::*;

#[tauri::command]
fn get_app_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit_hash = include_str!("../commit_hash.txt").trim();
    format!("{}-{}", version, commit_hash)
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| format!("Failed to get updater: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let current_version = app.package_info().version.to_string();
            Ok(Some(format!("{} -> {}", current_version, update.version)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Failed to check for updates: {}", e)),
    }
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| format!("Failed to get updater: {}", e))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| format!("Failed to install update: {}", e))
}

#[tauri::command]
async fn save_secrets(
    app: tauri::AppHandle,
    microsoft_client_id: String,
    supabase_url: String,
    supabase_key: String,
) -> Result<(), String> {
    let store = app.store("secrets.json").map_err(|e| e.to_string())?;
    store.set("microsoft_client_id", serde_json::Value::String(microsoft_client_id));
    store.set("supabase_url", serde_json::Value::String(supabase_url));
    store.set("supabase_key", serde_json::Value::String(supabase_key));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn is_secrets_configured(app: tauri::AppHandle) -> Result<bool, String> {
    let store = app.store("secrets.json").map_err(|e| e.to_string())?;

    let configured = |key: &str| -> bool {
        store.get(key).is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
    };

    Ok(configured("microsoft_client_id") && configured("supabase_url") && configured("supabase_key"))
}

pub struct CurseforgeConfig {
    pub api_key: Arc<str>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(move |app| {
            let store = app.store("secrets.json")?;

            let microsoft_client_id = store
                .get("microsoft_client_id")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| env!("MICROSOFT_CLIENT_ID").to_string());

            let supabase_url = store
                .get("supabase_url")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| env!("SUPABASE_URL").to_string());

            let supabase_key = store
                .get("supabase_key")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| env!("SUPABASE_SERVICE_KEY").to_string());

            let client_id = microsoft_client_id.clone();
            app.manage(AppConfig {
                microsoft_client_id,
                supabase_url,
                supabase_key,
            });

            let curseforge_api_key = env!("CURSEFORGE_API_KEY").to_string();
            app.manage(CurseforgeConfig {
                api_key: Arc::from(curseforge_api_key),
            });

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                let _ = crate::services::trash::TrashManager::clean_old_items(30);
                let account = AccountManager::get_active_account()
                    .map_err(|e| e.to_string())
                    .ok()
                    .flatten();
                if let Some(account) = account {
                    let _ = AccountManager::get_valid_token(&account.uuid, &client_id)
                        .await
                        .map_err(|e| e.to_string());
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                let app_handle = window.app_handle();
                let config = app_handle.state::<AppConfig>();
                let supabase_url = config.supabase_url.clone();
                let supabase_key = config.supabase_key.clone();
                let window = window.clone();

                tauri::async_runtime::spawn(async move {
                    let accounts = AccountManager::get_all_accounts()
                        .map_err(|e| e.to_string())
                        .ok();
                    if let Some(accounts) = accounts {
                        let service = FriendsService::new(&supabase_url, &supabase_key)
                            .map_err(|e| e.to_string())
                            .ok();
                        if let Some(service) = service {
                            for account in &accounts {
                                let _ = service
                                    .update_status(&account.uuid, FriendStatus::Offline, None)
                                    .await;
                            }
                        }
                    }
                    let _ = window.destroy();
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            check_for_updates,
            install_update,
            microsoft_login,
            microsoft_login_and_store,
            get_accounts,
            get_active_account,
            switch_account,
            remove_account,
            launch_instance_with_active_account,
            get_launch_token,
            refresh_account_token,
            send_friend_request,
            get_friend_requests,
            accept_friend_request,
            reject_friend_request,
            get_friends,
            remove_friend,
            update_user_status,
            update_specific_user_status,
            register_user_in_friends_system,
            upload_skin,
            reset_skin,
            get_current_skin,
            get_user_capes,
            equip_cape,
            remove_cape,
            load_recent_skins,
            save_recent_skin,
            get_minecraft_versions,
            get_minecraft_versions_with_metadata,
            get_minecraft_versions_by_type,
            get_supported_game_versions,
            install_minecraft,
            check_version_installed,
            get_fabric_versions,
            install_fabric,
            create_instance,
            get_instances,
            delete_instance,
            rename_instance,
            duplicate_instance,
            open_worlds_folder,
            open_world_folder,
            get_instance_worlds,
            delete_world,
            update_instance_fabric_loader,
            update_instance_neoforge_loader,
            update_instance_forge_loader,
            update_instance_minecraft_version,
            export_instance,
            get_neoforge_versions,
            get_neoforge_supported_game_versions,
            install_neoforge,
            get_forge_versions,
            get_forge_supported_game_versions,
            install_forge,
            get_all_screenshots,
            get_screenshot_data,
            delete_screenshot,
            open_screenshot,
            open_screenshots_folder,
            set_instance_icon,
            remove_instance_icon,
            get_instance_icon,
            launch_instance,
            launch_world,
            kill_instance,
            get_launcher_directory,
            open_instance_folder,
            search_mods,
            get_mod_details,
            get_mod_versions,
            download_mod,
            get_project_details,
            get_settings,
            save_settings,
            get_instance_settings,
            save_instance_settings,
            detect_java_installations,
            set_background,
            get_background,
            remove_background,
            open_directory,
            get_installed_mods,
            get_installed_mod_hashes,
            get_installed_mods_with_metadata,
            delete_mod,
            open_mods_folder,
            toggle_mod,
            get_modpack_versions,
            install_modpack,
            get_modpack_manifest,
            get_modpack_game_versions,
            install_modpack_from_file,
            get_modpack_name_from_file,
            detect_importable_instances,
            import_instance,
            get_installed_resourcepacks,
            download_resourcepack,
            delete_resourcepack,
            open_resourcepacks_folder,
            get_installed_shaderpacks,
            download_shaderpack,
            delete_shaderpack,
            open_shaderpacks_folder,
            get_servers,
            add_server,
            delete_server,
            update_server_status,
            launch_server,
            ping_server,
            reorder_servers,
            open_url,
            get_system_info,
            get_storage_usage,
            save_secrets,
            is_secrets_configured,
            search_curseforge_mods,
            get_curseforge_mod_files,
            get_curseforge_mod_details,
            download_curseforge_file,
            download_curseforge_file_temp,
            get_installed_resourcepacks_with_metadata,
            get_installed_shaderpacks_with_metadata,
            get_trash_size,
            empty_trash,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}