#![allow(dead_code)]

pub mod commands;
pub mod config;
pub mod db;
pub mod indexer;
pub mod kb;
pub mod ml;
pub mod search;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;

            tauri::async_runtime::block_on(async {
                // 初始化数据库
                let pool = db::init(&app_data).await.expect("db init failed");

                // 初始化知识库
                let kb_path = app_data.join("../../knowledge_base.json"); // app_data/knowledge_base.json
                let kb = kb::local::LocalKBProvider::load(&kb_path)
                    .unwrap_or_else(|_| kb::local::LocalKBProvider::empty());

                // 初始化搜索引擎（预加载向量索引）
                let engine = search::engine::SearchEngine::new(pool.clone(), Box::new(kb))
                    .await
                    .expect("search engine init failed");

                app.manage(pool);
                app.manage(Arc::new(engine) as commands::EngineState);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::add_images,
            commands::add_folder,
            commands::delete_image,
            commands::clear_gallery,
            commands::get_images,
            commands::get_image_meta,
            commands::relocate_image,
            commands::update_tags,
            commands::get_tag_suggestions,
            commands::copy_to_clipboard,
            commands::reveal_in_finder,
            commands::increment_use_count,
            commands::reindex_all,
            commands::check_file_statuses,
            commands::get_pending_tasks,
            commands::resume_pending_tasks,
            commands::clear_task_queue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
