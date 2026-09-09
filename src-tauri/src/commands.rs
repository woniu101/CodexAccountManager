use crate::{
    AppState, PendingLogin,
    app_server::{AppServerSession, current_codex_home},
    models::{DashboardState, LoginProgress, UserSettings, WindowPlacement},
    process_manager, settings, vault,
};
use serde_json::json;
use std::{fs, sync::MutexGuard, thread, time::Duration};
use tauri::{Emitter, PhysicalPosition, State, WebviewWindow};
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
        completed_account: None,
        completed_auth: None,
    });
    Ok(LoginProgress::Waiting { auth_url })
}

#[tauri::command]
pub fn poll_add_account(state: State<'_, AppState>) -> Result<LoginProgress, String> {
    let mut guard = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    let Some(pending) = guard.as_mut() else {
        return Ok(LoginProgress::Idle);
    };
    if let Some(account) = pending.completed_account.as_ref() {
        return Ok(LoginProgress::Duplicate {
            account: Box::new(account.clone()),
        });
    }
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
    if vault::has_account(&account.id)? {
        pending.completed_account = Some(account.clone());
        pending.completed_auth = Some(auth);
        return Ok(LoginProgress::Duplicate {
            account: Box::new(account),
        });
    }
    vault::store_account(&account, &auth)?;
    let result = LoginProgress::Completed {
        account: Box::new(account),
    };
    cleanup_pending(&mut guard);
    Ok(result)
}

#[tauri::command]
pub fn confirm_add_account(
    overwrite: bool,
    state: State<'_, AppState>,
) -> Result<LoginProgress, String> {
    let mut guard = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    if !overwrite {
        cleanup_pending(&mut guard);
        return Ok(LoginProgress::Idle);
    }
    let pending = guard.as_mut().ok_or("没有等待确认的重复账号")?;
    let account = pending
        .completed_account
        .take()
        .ok_or("重复账号信息不存在")?;
    let auth = pending.completed_auth.take().ok_or("重复账号凭据不存在")?;
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
    window: WebviewWindow,
) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    if process_manager::is_codex_cli_running()? {
        return Err("检测到正在运行的 Codex CLI；请先结束 CLI 任务再切换账号".to_string());
    }
    let _ = window.emit("switch-progress", "正在准备目标账号");
    let target = vault::target_auth(&account_id)?;
    let _ = window.emit("switch-progress", "正在关闭 Codex Desktop");
    let was_running = process_manager::stop_codex_desktop()?;
    let _ = window.emit("switch-progress", "正在保存当前账号");
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
    let _ = window.emit("switch-progress", "正在应用目标账号");
    if let Err(error) = vault::write_switch_journal(source_id.as_deref(), &account_id, "replaced") {
        rollback_switch(outgoing_auth.as_deref(), was_running);
        return Err(format!("记录切换状态失败，已恢复原账号：{error}"));
    }
    if was_running {
        let _ = window.emit("switch-progress", "正在重新启动 Codex Desktop");
        if let Err(error) = process_manager::start_codex_desktop() {
            rollback_switch(outgoing_auth.as_deref(), was_running);
            return Err(format!("Codex Desktop 启动失败，已恢复原账号：{error}"));
        }
        thread::sleep(Duration::from_secs(2));
    }
    let _ = window.emit("switch-progress", "正在验证目标账号");
    let verified = crate::app_server::query_home(current_codex_home()?)
        .is_ok_and(|account| account.id == account_id);
    if !verified {
        rollback_switch(outgoing_auth.as_deref(), was_running);
        let _ = window.emit("switch-progress", "切换失败，已恢复原账号");
        return Err("目标账号验证失败，已恢复原账号".to_string());
    }
    vault::clear_switch_journal();
    let _ = window.emit("switch-progress", "账号切换完成");
    Ok(DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    })
}

fn rollback_switch(outgoing_auth: Option<&[u8]>, restart_codex: bool) {
    if restart_codex {
        let _ = process_manager::stop_codex_desktop();
    }
    if let Some(outgoing) = outgoing_auth {
        let _ = vault::replace_live_auth(outgoing);
    }
    if restart_codex {
        let _ = process_manager::start_codex_desktop();
    }
    vault::clear_switch_journal();
}

#[tauri::command]
pub fn set_window_mode(
    window: WebviewWindow,
    mode: String,
    current_horizontal: String,
    current_vertical: String,
    expanded_height: f64,
) -> Result<WindowPlacement, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("无法确定当前显示器")?;
    let work_area = monitor.work_area();
    let right_edge = work_area.position.x + work_area.size.width as i32;
    let bottom_edge = work_area.position.y + work_area.size.height as i32;
    let (logical_width, logical_height) = match mode.as_str() {
        "hover" => (464.0, 96.0),
        "expanded" => (464.0, expanded_height.clamp(166.0, 380.0)),
        _ => (96.0, 96.0),
    };
    let scale = monitor.scale_factor();
    let width = (logical_width * scale).round() as u32;
    let height = (logical_height * scale).round() as u32;
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
    } else if mode == "expanded" {
        let space_up = old_bottom - work_area.position.y;
        let space_down = bottom_edge - position.y;
        space_up >= height as i32 || space_up >= space_down
    } else {
        old_bottom + height as i32 > bottom_edge
    };
    let x = if opens_left {
        old_right - width as i32
    } else {
        position.x.min(right_edge - width as i32)
    }
    .max(work_area.position.x);
    let y = if opens_up {
        old_bottom - height as i32
    } else {
        position.y.min(bottom_edge - height as i32)
    }
    .max(work_area.position.y);
    set_window_bounds(&window, x, y, width, height)?;
    Ok(WindowPlacement {
        horizontal: if opens_left { "left" } else { "right" }.to_string(),
        vertical: if opens_up { "up" } else { "down" }.to_string(),
    })
}

#[cfg(target_os = "windows")]
fn set_window_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    // SAFETY: `hwnd` belongs to this live Tauri window, dimensions are bounded by the
    // monitor work area, and SWP_NOZORDER means the null insert-after handle is ignored.
    let result = unsafe {
        SetWindowPos(
            hwnd.0,
            std::ptr::null_mut(),
            x,
            y,
            width as i32,
            height as i32,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
    };
    if result == 0 {
        Err(format!(
            "调整悬浮窗失败：{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn set_window_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use tauri::PhysicalSize;

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings() -> Result<UserSettings, String> {
    settings::load_user_settings()
}

#[tauri::command]
pub fn update_settings(settings: UserSettings) -> Result<UserSettings, String> {
    crate::settings::update_user_settings(settings)
}

#[tauri::command]
pub fn save_window_position(
    window: WebviewWindow,
    mode: String,
    horizontal: String,
) -> Result<(), String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let anchor = idle_anchor(position, size, scale, &mode, &horizontal);
    settings::save_window_position_at(&window, anchor)
}

fn idle_anchor(
    position: PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
    scale: f64,
    mode: &str,
    horizontal: &str,
) -> PhysicalPosition<i32> {
    let idle_size = (96.0 * scale).round() as i32;
    let x = if mode != "idle" && horizontal == "left" {
        position.x + size.width as i32 - idle_size
    } else {
        position.x
    };
    let y = if mode == "expanded" {
        position.y + size.height as i32 - idle_size
    } else {
        position.y
    };
    PhysicalPosition::new(x, y)
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

#[cfg(test)]
mod tests {
    use super::idle_anchor;
    use tauri::{PhysicalPosition, PhysicalSize};

    #[test]
    fn expanded_left_window_saves_the_idle_orb_anchor() {
        let anchor = idle_anchor(
            PhysicalPosition::new(100, 200),
            PhysicalSize::new(696, 360),
            1.5,
            "expanded",
            "left",
        );
        assert_eq!(anchor, PhysicalPosition::new(652, 416));
    }

    #[test]
    fn idle_window_saves_its_top_left_position() {
        let anchor = idle_anchor(
            PhysicalPosition::new(320, 240),
            PhysicalSize::new(144, 144),
            1.5,
            "idle",
            "right",
        );
        assert_eq!(anchor, PhysicalPosition::new(320, 240));
    }
}
