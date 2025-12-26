mod auth;
mod commands;
mod services;
mod utils;
mod models;

use commands::*;
use tauri::{AppHandle, Manager};

#[tauri::command]
async fn frontend_ready(app: AppHandle) {
    // Close splashscreen and show main window
    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        let _ = splashscreen.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Splash screen
            frontend_ready,
            
            // Authentication
            microsoft_login,
            
            // Multi-Account Authentication
            microsoft_login_and_store,
            get_accounts,
            get_active_account,
            switch_account,
            remove_account,
            launch_instance_with_active_account,
            
            // Minecraft versions
            get_minecraft_versions,
            install_minecraft,
            check_version_installed,
            
            // Fabric loader
            get_fabric_versions,
            install_fabric,
            
            // Instance management
            create_instance,
            get_instances,
            delete_instance,
            rename_instance,
            duplicate_instance,
            
            // Instance icons
            set_instance_icon,
            remove_instance_icon,
            get_instance_icon,
            
            // Launch
            launch_instance,
            
            // Launcher directory
            get_launcher_directory,
            open_instance_folder,
            
            // Modrinth API
            search_mods,
            get_mod_details,
            get_mod_versions,
            download_mod,
            
            // Settings
            get_settings,
            save_settings,
            get_instance_settings,
            save_instance_settings,
            detect_java_installations,

            // Mod Management
            get_installed_mods,
            delete_mod,
            open_mods_folder,
            toggle_mod,

            // Servers
            get_servers,
            add_server,
            delete_server,
            update_server_status,

            // Template Management
            create_template,
            get_templates,
            get_template,
            update_template,
            delete_template,
            create_template_from_instance,
            apply_template_to_instance,
            create_instance_from_template,

            // Debug
            generate_debug_report,
            save_debug_report,

            // System Info
            get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}