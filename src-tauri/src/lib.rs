// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod commands;
mod error;
mod events;
mod state;
mod typst_engine;
mod watcher;

use state::AppState;
use watcher::WatcherState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::default())
        .manage(WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            commands::project::open_project,
            commands::project::create_project,
            commands::project::close_project,
            commands::project::get_project_info,
            commands::project::set_main_file,
            commands::project::path_exists,
            commands::files::read_dir_tree,
            commands::files::read_file,
            commands::files::write_file,
            commands::files::file_mtime,
            commands::files::create_entry,
            commands::files::rename_entry,
            commands::files::delete_entry,
            commands::compile::compile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
