use crate::{
    AppState, PendingLogin,
    app_server::{AppServerSession, current_codex_home},
    models::{DashboardState, LoginProgress, WindowPlacement},
    process_manager, vault,
};
use serde_json::json;
use std::{fs, sync::MutexGuard, thread, time::Duration};
use tauri::{PhysicalPosition, PhysicalSize, State, WebviewWindow};
use uuid::Uuid;

#[tauri::command]
pub fn load_dashboard(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    Ok(DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    })
}

#[tauri::command]
pub fn import_current_account(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    vault::import_current_account()?;
    Ok(DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    })
}

#[tauri::command]
pub fn start_add_account(state: State<'_, AppState>) -> Result<LoginProgress, String> {
    let mut pending = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    if pending.is_some() {
        return Err("已有账号登录正在进行".to_string());
    }
    let home = vault::app_data_dir()?
        .join("temp")
        .join(format!("login-{}", Uuid::new_v4()));
    fs::create_dir_all(&home).map_err(|error| format!("创建登录目录失败：{error}"))?;
    fs::write(
        home.join("config.toml"),
        "cli_auth_credentials_store = \"file\"\n",
    )
    .map_err(|error| format!("写入登录配置失败：{error}"))?;
    let mut session = AppServerSession::start(home.clone())?;
    let result = session.request(
        "account/login/start",
        json!({ "type": "chatgpt", "useHostedLoginSuccessPage": true, "appBrand": "codex" }),
        Duration::from_secs(12),
    )?;
    let auth_url = result
        .get("authUrl")
        .and_then(serde_json::Value::as_str)
        .ok_or("登录接口没有返回 authUrl")?
        .to_string();
    let login_id = result
        .get("loginId")
        .and_then(serde_json::Value::as_str)
        .ok_or("登录接口没有返回 loginId")?
        .to_string();
    *pending = Some(PendingLogin {
        session,
        home,
        auth_url: auth_url.clone(),
        login_id,
    });
    Ok(LoginProgress::Waiting { auth_url })
}

#[tauri::command]
pub fn poll_add_account(state: State<'_, AppState>) -> Result<LoginProgress, String> {
    let mut guard = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    let Some(pending) = guard.as_mut() else {
        return Ok(LoginProgress::Idle);
    };
    let Some(notification) = pending.session.take_notification("account/login/completed") else {
        return Ok(LoginProgress::Waiting {
            auth_url: pending.auth_url.clone(),
        });
    };
    let params = notification.get("params").cloned().unwrap_or_default();
    let notification_id = params.get("loginId").and_then(serde_json::Value::as_str);
    if notification_id.is_some_and(|id| id != pending.login_id) {
        return Ok(LoginProgress::Waiting {
            auth_url: pending.auth_url.clone(),
        });
    }
    if !params
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        let message = params
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("浏览器授权失败")
            .to_string();
        cleanup_pending(&mut guard);
        return Ok(LoginProgress::Failed { message });
    }
    let account = pending.session.query_account()?;
    let auth = fs::read(pending.home.join("auth.json"))
        .map_err(|error| format!("读取新账号登录态失败：{error}"))?;
    vault::store_account(&account, &auth)?;
    let result = LoginProgress::Completed {
        account: Box::new(account),
    };
    cleanup_pending(&mut guard);
    Ok(result)
}

#[tauri::command]
pub fn cancel_add_account(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    if let Some(pending) = guard.as_mut() {
        let _ = pending.session.request(
            "account/login/cancel",
            json!({ "loginId": pending.login_id }),
            Duration::from_secs(5),
        );
    }
    cleanup_pending(&mut guard);
    Ok(())
}

#[tauri::command]
pub fn switch_account(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    let target = vault::target_auth(&account_id)?;
    let was_running = process_manager::stop_codex_desktop()?;
    let outgoing_auth = fs::read(vault::auth_path()?).ok();
    let source_id = match vault::save_live_auth_to_vault() {
        Ok(source_id) => source_id,
        Err(error) => {
            if was_running {
                let _ = process_manager::start_codex_desktop();
            }
            return Err(error);
        }
    };
    if let Err(error) = vault::write_switch_journal(source_id.as_deref(), &account_id, "prepared") {
        if was_running {
            let _ = process_manager::start_codex_desktop();
        }
        return Err(error);
    }

    if let Err(error) = vault::replace_live_auth(&target) {
        vault::clear_switch_journal();
        if was_running {
            let _ = process_manager::start_codex_desktop();
        }
        return Err(error);
    }
    vault::write_switch_journal(source_id.as_deref(), &account_id, "replaced")?;
    if was_running {
        process_manager::start_codex_desktop()?;
        thread::sleep(Duration::from_secs(2));
    }
    let verified = crate::app_server::query_home(current_codex_home()?)
        .is_ok_and(|account| account.id == account_id);
    if !verified {
        if was_running {
            let _ = process_manager::stop_codex_desktop();
        }
        if let Some(outgoing) = outgoing_auth {
            let _ = vault::replace_live_auth(&outgoing);
        }
        if was_running {
            let _ = process_manager::start_codex_desktop();
        }
        vault::clear_switch_journal();
        return Err("目标账号验证失败，已恢复原账号".to_string());
    }
    vault::clear_switch_journal();
    Ok(DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    })
}

#[tauri::command]
pub fn set_window_mode(
    window: WebviewWindow,
    mode: String,
    current_horizontal: String,
    current_vertical: String,
) -> Result<WindowPlacement, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("无法确定当前显示器")?;
    let right_edge = monitor.position().x + monitor.size().width as i32;
    let bottom_edge = monitor.position().y + monitor.size().height as i32;
    let (width, height) = match mode.as_str() {
        "hover" => (440_u32, 100_u32),
        "expanded" => (440_u32, 380_u32),
        _ => (92_u32, 92_u32),
    };
    let old_right = position.x + size.width as i32;
    let old_bottom = position.y + size.height as i32;
    let collapsing = mode == "idle" && (size.width > width || size.height > height);
    let opens_left = if collapsing {
        current_horizontal == "left"
    } else {
        position.x + width as i32 > right_edge
    };
    let opens_up = if collapsing {
        current_vertical == "up"
    } else {
        mode == "expanded" || old_bottom + height as i32 > bottom_edge
    };
    let x = if opens_left {
        old_right - width as i32
    } else {
        position.x
    };
    let y = if opens_up {
        old_bottom - height as i32
    } else {
        position.y
    };
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    Ok(WindowPlacement {
        horizontal: if opens_left { "left" } else { "right" }.to_string(),
        vertical: if opens_up { "up" } else { "down" }.to_string(),
    })
}

fn cleanup_pending(guard: &mut MutexGuard<'_, Option<PendingLogin>>) {
    if let Some(pending) = guard.take() {
        let _ = fs::remove_dir_all(pending.home);
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
