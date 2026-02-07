//! noslop GUI - Tauri backend
//!
//! This module provides the Tauri command handlers that bridge
//! the Svelte frontend with the noslop library.

mod commands;
mod dto;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_reviews,
            commands::get_review,
            commands::get_diff,
            commands::start_review,
            commands::add_comment,
            commands::resolve_comment,
            commands::close_review,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
