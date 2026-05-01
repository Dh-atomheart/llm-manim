mod commands;
mod services;
mod state;
mod types;

use crate::{commands::workspace::restore_workspace_state, services::queue, state::AppState};
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
                let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                state.set_queue_sender(queue_tx).await;

                let worker_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    queue::run_queue_worker(queue_rx, |job_id| {
                        let worker_handle = worker_handle.clone();
                        async move {
                            queue::process_job(worker_handle, job_id).await;
                        }
                    })
                    .await;
                });

                queue::recover_running_jobs(&app_handle).await;
                queue::requeue_queued_jobs(&app_handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping_backend,
            commands::workspace::get_workspace_status,
            commands::workspace::initialize_workspace,
            commands::workspace::check_runtime,
            commands::settings::get_generation_settings,
            commands::settings::update_generation_settings,
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::delete_project,
            commands::provider::list_provider_configs,
            commands::provider::save_provider_config,
            commands::provider::delete_provider_config,
            commands::provider::test_provider_config,
            commands::job::submit_prompt_job,
            commands::job::get_job,
            commands::job::list_project_jobs,
            commands::job::cancel_job,
            commands::job::delete_job,
            commands::job::retry_job,
            commands::job::get_job_logs,
            commands::job::get_render_artifact,
            commands::job::get_video_file_url,
            commands::job::open_render_artifact
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
