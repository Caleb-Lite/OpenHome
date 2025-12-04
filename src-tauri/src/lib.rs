mod commands;
mod deprecated;
mod error;
mod menu;
mod pkm_storage;
mod plugin;
mod saves;
mod startup;
mod state;
mod util;
mod versioning;

use std::env;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::error::Error;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if let Err(launch_error) = startup::run_app_startup(app) {
                match launch_error {
                    Error::OutdatedVersion { .. } => app.handle().exit(1),
                    _ => {
                        fatal_error_dialog(app, "OpenHome Failed to Launch", launch_error);
                    }
                }
            }

            let lookup_state = match state::LookupState::load_from_storage(app.handle()) {
                Ok(lookup) => lookup,
                Err(err) => {
                    fatal_error_dialog(app, "OpenHome Failed to Launch - Lookup File Error", err)
                }
            };
            app.manage(lookup_state);

            let pokedex_state = match state::PokedexState::load_from_storage(app.handle()) {
                Ok(pokedex) => pokedex,
                Err(err) => {
                    fatal_error_dialog(app, "OpenHome Failed to Launch - Pokedex File Error", err)
                }
            };
            app.manage(pokedex_state);

            app.manage(state::AppState::default());

            match menu::create_menu(app) {
                Ok(menu) => {
                    let _ = app.set_menu(menu);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error creating menu: {e}");
                    Err(e)
                }
            }
        })
        .on_menu_event(|app_handle, event| {
            menu::handle_menu_event(app_handle, event);
        })
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_file_bytes,
            commands::get_file_created,
            commands::get_image_data,
            commands::get_storage_file_json,
            commands::write_storage_file_json,
            commands::write_file_bytes,
            commands::write_storage_file_bytes,
            commands::get_ohpkm_files,
            commands::delete_storage_files,
            commands::start_transaction,
            commands::rollback_transaction,
            commands::commit_transaction,
            commands::find_suggested_saves,
            commands::set_app_theme,
            commands::validate_recent_saves,
            commands::download_plugin,
            commands::list_installed_plugins,
            commands::load_plugin_code,
            commands::delete_plugin,
            commands::handle_windows_accellerator,
            commands::open_directory,
            pkm_storage::load_banks,
            pkm_storage::write_banks,
            state::get_lookups,
            state::update_lookups,
            state::get_pokedex,
            state::update_pokedex,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(target_os = "linux"))]
fn fatal_error_dialog(app: &tauri::App, title: &'static str, error: Error) -> ! {
    app.dialog()
        .message(error.to_string())
        .title(title)
        .kind(MessageDialogKind::Error)
        .blocking_show();

    app.handle().cleanup_before_exit();
    std::process::exit(1)
}

#[cfg(target_os = "linux")]
fn fatal_error_dialog(app: &tauri::App, title: &'static str, error: Error) -> ! {
    let dialog_handle = app.handle().clone();
    let (tx, rx) = std::sync::mpsc::channel();

    // Because Linux won't show a blocking dialog before the app initializes, this fakes a blocking dialog by
    // showing a nonblocking dialog, and using a channel to wait from the outer thread.
    let _ = app.run_on_main_thread(move || {
        dialog_handle
            .dialog()
            .message(error.to_string())
            .title(title)
            .kind(MessageDialogKind::Error)
            .show(move |_| {
                let _ = tx.send(());
            });
    });

    let _ = rx.recv();

    app.handle().cleanup_before_exit();
    std::process::exit(1)
}
