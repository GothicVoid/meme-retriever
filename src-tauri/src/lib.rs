mod commands;
mod config;
mod db;
mod indexer;
mod kb;
mod ml;
mod search;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            tauri::async_runtime::block_on(async {
                let pool = db::init(&app_data).await.expect("db init failed");
                app.manage(pool);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::add_images,
            commands::delete_image,
            commands::get_images,
            commands::update_tags,
            commands::get_tag_suggestions,
            commands::copy_to_clipboard,
            commands::reveal_in_finder,
            commands::increment_use_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
