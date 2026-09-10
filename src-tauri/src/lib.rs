mod app_server;
mod commands;
mod models;
mod process_manager;
mod settings;
mod tray_menu;
mod vault;

use app_server::AppServerSession;
use std::{path::PathBuf, sync::Mutex, time::Instant};
use tauri::{
    Emitter, Manager,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub struct PendingLogin {
    session: AppServerSession,
    home: PathBuf,
    auth_url: String,
    login_id: String,
    started_at: Instant,
    completed_account: Option<models::ManagedAccount>,
    completed_auth: Option<Vec<u8>>,
}

pub struct AppState {
    pending_login: Mutex<Option<PendingLogin>>,
    operation_lock: Mutex<()>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Release builds launched from Codex's terminal inherit the Codex Desktop
    // process job. Relaunch through Explorer so closing/restarting Codex during
    // an account switch cannot terminate this manager as a child process.
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if process_manager::relaunch_outside_codex_tree().unwrap_or(false) {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            pending_login: Mutex::new(None),
            operation_lock: Mutex::new(()),
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = settings::restore_window_position(&window);
            }
            if vault::has_switch_journal() {
                let was_running = process_manager::stop_codex_desktop().unwrap_or(false);
                let _ = vault::recover_incomplete_switch();
                if was_running {
                    let _ = process_manager::start_codex_desktop();
                }
            }
            vault::cleanup_stale_temp_dirs();

            let menu = tray_menu::build(app.handle(), &[], false)?;
            let tray_handle = app.handle().clone();

            TrayIconBuilder::with_id(tray_menu::TRAY_ID)
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("application icon"),
                )
                .tooltip("Codex Account Manager")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if let Some(account_id) = event
                        .id
                        .as_ref()
                        .strip_prefix(tray_menu::SWITCH_ACCOUNT_PREFIX)
                    {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("switch-account-requested", account_id.to_string());
                        }
                        return;
                    }

                    match event.id.as_ref() {
                        "toggle-visibility" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let visible = window.is_visible().unwrap_or(true);
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                let accounts = tray_accounts_from_cache();
                                let _ = tray_menu::update(app, &accounts, !visible);
                            }
                        }
                        "refresh" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("refresh-requested", ());
                            }
                        }
                        "add" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("add-account-requested", ());
                            }
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("settings-requested", ());
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                        && let Some(window) = tray_handle.get_webview_window("main")
                    {
                        let visible = window.is_visible().unwrap_or(true);
                        if visible {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let accounts = tray_accounts_from_cache();
                        let _ = tray_menu::update(&tray_handle, &accounts, !visible);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_dashboard,
            commands::import_current_account,
            commands::start_add_account,
            commands::poll_add_account,
            commands::confirm_add_account,
            commands::cancel_add_account,
            commands::switch_account,
            commands::remove_account,
            commands::set_window_mode,
            commands::get_settings,
            commands::update_settings,
            commands::save_window_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Account Manager");
}

fn tray_accounts_from_cache() -> Vec<models::ManagedAccount> {
    vault::load_accounts()
        .unwrap_or_default()
        .into_iter()
        .map(|meta| {
            meta.cached.unwrap_or_else(|| models::ManagedAccount {
                id: meta.id,
                email: meta.email,
                alias: meta.alias,
                plan_type: meta.plan_type,
                ..models::ManagedAccount::default()
            })
        })
        .collect()
}
