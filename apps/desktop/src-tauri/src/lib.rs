mod commands;
mod state;

use chrono::Utc;
use taskboard_application::{App, SystemClock};
use taskboard_store_sqlite::{open_db, resolve_data_dir, SqliteStore};

use state::DesktopState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = resolve_data_dir(None, std::env::var_os("TASKBOARD_DATA_DIR").as_deref());
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = open_db(&data_dir).await.map_err(|err| err.to_string())?;
                let store = SqliteStore::new(pool, &data_dir);
                let board = App::new(store, SystemClock);
                board
                    .purge_expired_trash(Utc::now())
                    .await
                    .map_err(|err| err.to_string())?;
                handle.manage(DesktopState {
                    app: tokio::sync::Mutex::new(board),
                });
                Ok::<(), String>(())
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project_add,
            commands::project_list,
            commands::project_update,
            commands::project_reorder,
            commands::project_archive,
            commands::project_delete,
            commands::project_restore,
            commands::project_note_set,
            commands::task_create,
            commands::task_list,
            commands::task_show,
            commands::task_update,
            commands::task_move,
            commands::task_reorder,
            commands::task_urgent,
            commands::task_delete,
            commands::task_restore,
            commands::task_note_set,
            commands::link_add,
            commands::link_remove,
            commands::run_start,
            commands::run_patch,
            commands::trash_list,
            commands::undo,
            commands::sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Taskboard");
}
