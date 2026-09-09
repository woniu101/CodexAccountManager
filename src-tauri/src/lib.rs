mod app_server;
mod commands;
mod models;
mod process_manager;
mod settings;
mod vault;

use app_server::AppServerSession;
use std::{path::PathBuf, sync::Mutex};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub struct PendingLogin {
    session: AppServerSession,
    home: PathBuf,
    auth_url: String,
    login_id: String,
    completed_account: Option<models::ManagedAccount>,
    completed_auth: Option<Vec<u8>>,
}

pub struct AppState {
    pending_login: Mutex<Option<PendingLogin>>,
    operation_lock: Mutex<()>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            pending_login: Mutex::new(None),
            operation_lock: Mutex::new(()),
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = settings::restore_window_position(&window);
                window.show()?;
            }
            if vault::has_switch_journal() {
                let was_running = process_manager::stop_codex_desktop().unwrap_or(false);
                let _ = vault::recover_incomplete_switch();
                if was_running {
                    let _ = process_manager::start_codex_desktop();
                }
            }
            vault::cleanup_stale_temp_dirs();

            let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "隐藏", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
            let add = MenuItem::with_id(app, "add", "添加账号", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &refresh, &add, &settings, &quit])?;
            let tray_handle = app.handle().clone();

            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("application icon"),
                )
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
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
                })
                .on_tray_icon_event(move |_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                        && let Some(window) = tray_handle.get_webview_window("main")
                    {
                        let _ = window.show();
                        let _ = window.set_focus();
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
            commands::set_window_mode,
            commands::get_settings,
            commands::update_settings,
            commands::save_window_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Account Manager");
}
