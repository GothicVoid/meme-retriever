#![allow(dead_code)]

pub mod commands;
pub mod config;
pub mod db;
pub mod image_io;
pub mod indexer;
pub mod kb;
pub mod ml;
pub mod search;

use std::sync::Arc;
use tauri::{Manager, WindowEvent};

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
                let kb_path = kb::maintenance::resolve_default_kb_path();
                let kb_file =
                    kb::maintenance::KnowledgeBaseFile::load(&kb_path).unwrap_or_default();
                let kb = kb::local::LocalKBProvider::load(&kb_path)
                    .unwrap_or_else(|_| kb::local::LocalKBProvider::empty());
                let example_image_index =
                    kb::example_index::ExampleImageIndex::from_knowledge_base(&kb_file, &kb_path);

                // 初始化搜索引擎（预加载向量索引）
                let engine = search::engine::SearchEngine::new(pool.clone(), Box::new(kb))
                    .await
                    .expect("search engine init failed");
                engine.set_example_image_index(example_image_index);

                app.manage(pool);
                app.manage(Arc::new(engine) as commands::EngineState);
            });

            if let Some(window) = app.get_webview_window("main") {
                let prefs = commands::load_window_preferences_from_dir(&app_data);
                commands::save_window_mode_to_dir(&app_data, "sidebar")?;
                commands::apply_window_layout_to_window(&window, "sidebar", &prefs)?;

                let app_data_for_events = app_data.clone();
                let event_window = window.clone();
                window.on_window_event(move |event| {
                    let snapshot = match event {
                        WindowEvent::Moved(position) => Some(commands::WindowSnapshot {
                            x: position.x as f64,
                            y: position.y as f64,
                            width: 0.0,
                            height: 0.0,
                        }),
                        WindowEvent::Resized(size) => Some(commands::WindowSnapshot {
                            x: 0.0,
                            y: 0.0,
                            width: size.width as f64,
                            height: size.height as f64,
                        }),
                        _ => None,
                    };

                    if snapshot.is_none() {
                        return;
                    }

                    let prefs = commands::load_window_preferences_from_dir(&app_data_for_events);
                    let Ok(position) = event_window.outer_position() else {
                        return;
                    };
                    let Ok(size) = event_window.outer_size() else {
                        return;
                    };
                    let snapshot = commands::WindowSnapshot {
                        x: position.x as f64,
                        y: position.y as f64,
                        width: size.width as f64,
                        height: size.height as f64,
                    };
                    let _ = commands::update_window_snapshot_in_dir(
                        &app_data_for_events,
                        &prefs.mode,
                        snapshot,
                    );
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::get_home_state,
            commands::get_latest_import_summary,
            commands::get_import_batch_failures,
            commands::delete_search_history,
            commands::clear_search_history,
            commands::record_search_history,
            commands::import_entries,
            commands::add_images,
            commands::add_folder,
            commands::delete_image,
            commands::clear_gallery,
            commands::clear_missing_images,
            commands::get_images,
            commands::get_image_count,
            commands::get_image_meta,
            commands::relocate_image,
            commands::update_tags,
            commands::get_tag_suggestions,
            commands::copy_to_clipboard,
            commands::reveal_in_finder,
            commands::increment_use_count,
            commands::reindex_all,
            commands::reindex_gif_indexes,
            commands::check_file_statuses,
            commands::get_pending_tasks,
            commands::resume_pending_tasks,
            commands::clear_task_queue,
            commands::apply_window_layout,
            commands::save_window_preferences,
            commands::show_main_window,
            commands::kb_get_state,
            commands::kb_validate_entries,
            commands::kb_save_entries,
            commands::kb_import_example_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
