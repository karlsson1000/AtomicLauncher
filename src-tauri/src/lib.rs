mod auth;
mod commands;
mod services;
mod utils;
mod models;

use commands::{
    // Auth commands
    microsoft_login,
    microsoft_login_and_store,
    get_accounts,
    get_active_account,
    switch_account,
    remove_account,
    
    // Instance commands
    create_instance,
    get_instances,
    delete_instance,
    rename_instance,
    duplicate_instance,
    launch_instance,
    kill_instance,
    launch_instance_with_active_account,
    set_instance_icon,
    remove_instance_icon,
    get_instance_icon,
    get_launcher_directory,
    open_instance_folder,
    open_worlds_folder,
    open_world_folder,
    get_instance_worlds,
    delete_world,
    
    // Version commands
    get_minecraft_versions,
    get_minecraft_versions_with_metadata,
    get_minecraft_versions_by_type,
    get_supported_game_versions,
    install_minecraft,
    check_version_installed,
    get_fabric_versions,
    install_fabric,
    
    // Mod commands
    get_installed_mods,
    delete_mod,
    open_mods_folder,
    toggle_mod,
    search_mods,
    get_mod_details,
    get_mod_versions,
    download_mod,
    get_project_details,
    get_favorite_mods,
    save_favorite_mods,
    
    // Modpack commands
    get_modpack_versions,
    install_modpack,
    get_modpack_manifest,
    get_modpack_game_versions,
    install_modpack_from_file,
    get_modpack_name_from_file,
    
    // Server commands
    get_servers,
    add_server,
    delete_server,
    update_server_status,
    launch_server,
    
    // Settings commands
    get_settings,
    save_settings,
    get_instance_settings,
    save_instance_settings,
    detect_java_installations,
    set_sidebar_background,
    get_sidebar_background,
    remove_sidebar_background,
    
    // Template commands
    create_template,
    get_templates,
    get_template,
    update_template,
    delete_template,
    create_template_from_instance,
    apply_template_to_instance,
    create_instance_from_template,
    export_template,
    import_template,
    
    // Skin commands
    upload_skin,
    reset_skin,
    get_current_skin,
    get_user_capes,
    equip_cape,
    remove_cape,
    
    // System commands
    get_system_info,
    generate_debug_report,
    save_debug_report,
    open_url,
};

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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // Splash screen
            frontend_ready,
            
            // Authentication
            microsoft_login,
            microsoft_login_and_store,
            get_accounts,
            get_active_account,
            switch_account,
            remove_account,
            launch_instance_with_active_account,
            
            // Skin Management
            upload_skin,
            reset_skin,
            get_current_skin,
            get_user_capes,
            equip_cape,
            remove_cape,
            
            // Minecraft versions
            get_minecraft_versions,
            get_minecraft_versions_with_metadata,
            get_minecraft_versions_by_type,
            get_supported_game_versions,
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
            open_worlds_folder,
            open_world_folder,
            get_instance_worlds,
            delete_world,
            
            // Instance icons
            set_instance_icon,
            remove_instance_icon,
            get_instance_icon,
            
            // Launch
            launch_instance,
            kill_instance,
            
            // Launcher directory
            get_launcher_directory,
            open_instance_folder,
            
            // Modrinth API
            search_mods,
            get_mod_details,
            get_mod_versions,
            download_mod,
            get_project_details,
            
            // Settings
            get_settings,
            save_settings,
            get_instance_settings,
            save_instance_settings,
            detect_java_installations,
            set_sidebar_background,
            get_sidebar_background,
            remove_sidebar_background,

            // Mod Management
            get_installed_mods,
            delete_mod,
            open_mods_folder,
            toggle_mod,
            get_favorite_mods,
            save_favorite_mods,

            // Modpacks
            get_modpack_versions,
            install_modpack,
            get_modpack_manifest,
            get_modpack_game_versions,
            install_modpack_from_file,
            get_modpack_name_from_file,

            // Servers
            get_servers,
            add_server,
            delete_server,
            update_server_status,
            launch_server,

            // Template Management
            create_template,
            get_templates,
            get_template,
            update_template,
            delete_template,
            create_template_from_instance,
            apply_template_to_instance,
            create_instance_from_template,
            export_template,
            import_template,

            // Open links
            open_url,

            // Debug
            generate_debug_report,
            save_debug_report,

            // System Info
            get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}