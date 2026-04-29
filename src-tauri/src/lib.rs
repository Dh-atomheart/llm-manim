mod commands;
mod services;
mod state;
mod types;

use crate::{commands::workspace::restore_workspace_state, state::AppState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();

            tauri::async_runtime::block_on(async move {
                restore_workspace_state(&app_handle, &state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping_backend,
            commands::workspace::get_workspace_status,
            commands::workspace::initialize_workspace,
            commands::workspace::check_runtime,
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::delete_project,
            commands::provider::list_provider_configs,
            commands::provider::save_provider_config,
            commands::provider::delete_provider_config,
            commands::provider::test_provider_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
