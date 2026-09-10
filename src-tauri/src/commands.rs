use crate::{
    AppState, PendingLogin,
    app_server::{AppServerSession, current_codex_home},
    models::{DashboardState, LoginProgress, ManagedAccount, UserSettings, WindowPlacement},
    process_manager, settings, vault,
};
use serde_json::json;
use std::{
    fs,
    sync::MutexGuard,
    thread,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalRect, State, WebviewWindow};
use uuid::Uuid;

#[tauri::command]
pub async fn load_dashboard(
    state: State<'_, AppState>,
    window: WebviewWindow,
) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    let dashboard = DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    };
    sync_tray_menu(&window, &dashboard);
    Ok(dashboard)
}

#[tauri::command]
pub async fn import_current_account(
    state: State<'_, AppState>,
    window: WebviewWindow,
) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    vault::import_current_account()?;
    let dashboard = DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    };
    sync_tray_menu(&window, &dashboard);
    Ok(dashboard)
}

#[tauri::command]
pub async fn start_add_account(state: State<'_, AppState>) -> Result<LoginProgress, String> {
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
    let prepared = (|| -> Result<(AppServerSession, String, String), String> {
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
        Ok((session, auth_url, login_id))
    })();
    let (session, auth_url, login_id) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            let _ = fs::remove_dir_all(&home);
            return Err(error);
        }
    };
    *pending = Some(PendingLogin {
        session,
        home,
        auth_url: auth_url.clone(),
        login_id,
        started_at: Instant::now(),
        completed_account: None,
        completed_auth: None,
    });
    Ok(LoginProgress::Waiting { auth_url })
}

#[tauri::command]
pub async fn poll_add_account(state: State<'_, AppState>) -> Result<LoginProgress, String> {
    let mut guard = state.pending_login.lock().map_err(|_| "登录状态锁已损坏")?;
    let Some(pending) = guard.as_mut() else {
        return Ok(LoginProgress::Idle);
    };
    if pending.completed_account.is_none()
        && pending.started_at.elapsed() >= Duration::from_secs(300)
    {
        let _ = pending.session.request(
            "account/login/cancel",
            json!({ "loginId": pending.login_id }),
            Duration::from_secs(3),
        );
        cleanup_pending(&mut guard);
        return Ok(LoginProgress::Failed {
            message: "授权等待已超过 5 分钟，请重新添加账号".to_string(),
        });
    }
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
    let completed = (|| -> Result<(ManagedAccount, Vec<u8>, bool), String> {
        let account = pending.session.query_account()?;
        let auth = fs::read(pending.home.join("auth.json"))
            .map_err(|error| format!("读取新账号登录态失败：{error}"))?;
        let duplicate = vault::has_account(&account.id)?;
        Ok((account, auth, duplicate))
    })();
    let (account, auth, duplicate) = match completed {
        Ok(completed) => completed,
        Err(message) => {
            cleanup_pending(&mut guard);
            return Ok(LoginProgress::Failed { message });
        }
    };
    if duplicate {
        pending.completed_account = Some(account.clone());
        pending.completed_auth = Some(auth);
        return Ok(LoginProgress::Duplicate {
            account: Box::new(account),
        });
    }
    if let Err(message) = vault::store_account(&account, &auth) {
        cleanup_pending(&mut guard);
        return Ok(LoginProgress::Failed { message });
    }
    let result = LoginProgress::Completed {
        account: Box::new(account),
    };
    cleanup_pending(&mut guard);
    Ok(result)
}

#[tauri::command]
pub async fn confirm_add_account(
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
pub async fn cancel_add_account(state: State<'_, AppState>) -> Result<(), String> {
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
pub async fn switch_account(
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
    let dashboard = DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    };
    sync_tray_menu(&window, &dashboard);
    Ok(dashboard)
}

#[tauri::command]
pub async fn remove_account(
    state: State<'_, AppState>,
    account_id: String,
    window: WebviewWindow,
) -> Result<DashboardState, String> {
    let _guard = state.operation_lock.lock().map_err(|_| "操作锁已损坏")?;
    vault::remove_account(&account_id)?;
    let dashboard = DashboardState {
        accounts: vault::refresh_all_accounts()?,
        codex_running: process_manager::is_codex_running(),
        refreshed_at: Some(unix_now()),
    };
    sync_tray_menu(&window, &dashboard);
    Ok(dashboard)
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
    current_mode: String,
    current_horizontal: String,
    current_vertical: String,
    expanded_height: f64,
) -> Result<WindowPlacement, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let current_scale = window.scale_factor().map_err(|error| error.to_string())?;
    let current_inset = anchor_inset(&current_mode, current_scale);
    let anchor = PhysicalPosition::new(
        position.x
            + if current_horizontal == "left" {
                size.width as i32 - current_inset
            } else {
                current_inset
            },
        position.y
            + if current_vertical == "up" {
                size.height as i32 - current_inset
            } else {
                current_inset
            },
    );
    let monitors = window
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let monitor = monitors
        .into_iter()
        .find(|monitor| point_in_work_area(anchor, monitor.work_area()))
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("无法确定当前显示器")?;
    let work_area = monitor.work_area();
    let left_edge = work_area.position.x;
    let top_edge = work_area.position.y;
    let right_edge = work_area.position.x + work_area.size.width as i32;
    let bottom_edge = work_area.position.y + work_area.size.height as i32;
    let expanded_height = expanded_height.clamp(140.0, 420.0);
    let scale = monitor.scale_factor();
    let decision_width = (424.0 * scale).round() as i32;
    let decision_height = (expanded_height * scale).round() as i32;
    let decision_inset = anchor_inset("expanded", scale);
    let margin = (12.0 * scale).round() as i32;
    let opens_left = choose_negative_direction(
        anchor.x,
        left_edge,
        right_edge,
        decision_width,
        decision_inset,
        margin,
        false,
    );
    let opens_up = choose_negative_direction(
        anchor.y,
        top_edge,
        bottom_edge,
        decision_height,
        decision_inset,
        margin,
        true,
    );
    let (visible_logical_width, visible_logical_height) = match mode.as_str() {
        "hover" => (424.0, 88.0),
        "expanded" => (424.0, expanded_height),
        _ => (88.0, 88.0),
    };
    let visible_width = (visible_logical_width * scale).round() as u32;
    let visible_height = (visible_logical_height * scale).round() as u32;

    #[cfg(target_os = "windows")]
    let (width, height) = (
        (424.0 * scale).round() as u32,
        (420.0 * scale).round() as u32,
    );
    #[cfg(not(target_os = "windows"))]
    let (width, height) = (visible_width, visible_height);

    let target_inset = anchor_inset(&mode, scale);
    let target_x = if opens_left {
        anchor.x - (width as i32 - target_inset)
    } else {
        anchor.x - target_inset
    };
    let target_y = if opens_up {
        anchor.y - (height as i32 - target_inset)
    } else {
        anchor.y - target_inset
    };

    #[cfg(target_os = "windows")]
    let (x, y) = (target_x, target_y);
    #[cfg(not(target_os = "windows"))]
    let (x, y) = (
        target_x.clamp(left_edge, (right_edge - width as i32).max(left_edge)),
        target_y.clamp(top_edge, (bottom_edge - height as i32).max(top_edge)),
    );

    if position.x != x || position.y != y || size.width != width || size.height != height {
        set_window_bounds(&window, x, y, width, height)?;
    }
    set_window_region(
        &window,
        if opens_left {
            width as i32 - visible_width as i32
        } else {
            0
        },
        if opens_up {
            height as i32 - visible_height as i32
        } else {
            0
        },
        visible_width,
        visible_height,
        (80.0 * scale).round() as i32,
    )?;
    Ok(WindowPlacement {
        horizontal: if opens_left { "left" } else { "right" }.to_string(),
        vertical: if opens_up { "up" } else { "down" }.to_string(),
    })
}

fn anchor_inset(_mode: &str, scale: f64) -> i32 {
    (44.0 * scale).round() as i32
}

fn point_in_work_area(point: PhysicalPosition<i32>, area: &PhysicalRect<i32, u32>) -> bool {
    point.x >= area.position.x
        && point.x < area.position.x + area.size.width as i32
        && point.y >= area.position.y
        && point.y < area.position.y + area.size.height as i32
}

fn choose_negative_direction(
    anchor: i32,
    start: i32,
    end: i32,
    target_size: i32,
    anchor_inset: i32,
    margin: i32,
    prefer_negative: bool,
) -> bool {
    let negative_extent = target_size - anchor_inset;
    let positive_extent = target_size - anchor_inset;
    let fits_negative =
        anchor - negative_extent >= start + margin && anchor + anchor_inset <= end - margin;
    let fits_positive =
        anchor - anchor_inset >= start + margin && anchor + positive_extent <= end - margin;
    match (fits_negative, fits_positive) {
        (true, true) => prefer_negative,
        (true, false) => true,
        (false, true) => false,
        (false, false) => anchor - start > end - anchor,
    }
}

#[cfg(target_os = "windows")]
fn set_window_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOZORDER, SetWindowPos,
    };

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
            SWP_NOACTIVATE | SWP_NOCOPYBITS | SWP_NOZORDER,
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

#[cfg(target_os = "windows")]
fn set_window_region(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    corner_diameter: i32,
) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    // SAFETY: the region is created with bounded window-relative dimensions. On a
    // successful SetWindowRgn call Windows owns and eventually deletes the HRGN.
    let region = unsafe {
        CreateRoundRectRgn(
            x,
            y,
            x + width as i32,
            y + height as i32,
            corner_diameter,
            corner_diameter,
        )
    };
    if region.is_null() {
        return Err(format!(
            "创建悬浮窗可见区域失败：{}",
            std::io::Error::last_os_error()
        ));
    }
    let result = unsafe { SetWindowRgn(hwnd.0, region, 1) };
    if result == 0 {
        unsafe { DeleteObject(region) };
        Err(format!(
            "更新悬浮窗可见区域失败：{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn set_window_region(
    _window: &WebviewWindow,
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
    _corner_diameter: i32,
) -> Result<(), String> {
    Ok(())
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
    vertical: String,
) -> Result<(), String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let anchor = idle_anchor(position, size, scale, &mode, &horizontal, &vertical);
    settings::save_window_position_at(&window, anchor)
}

fn idle_anchor(
    position: PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
    scale: f64,
    mode: &str,
    horizontal: &str,
    vertical: &str,
) -> PhysicalPosition<i32> {
    let inset = anchor_inset(mode, scale);
    let idle_half = anchor_inset("idle", scale);
    let center_x = if horizontal == "left" {
        position.x + size.width as i32 - inset
    } else {
        position.x + inset
    };
    let center_y = if vertical == "up" {
        position.y + size.height as i32 - inset
    } else {
        position.y + inset
    };
    PhysicalPosition::new(center_x - idle_half, center_y - idle_half)
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

fn sync_tray_menu(window: &WebviewWindow, dashboard: &DashboardState) {
    let visible = window.is_visible().unwrap_or(true);
    let _ = crate::tray_menu::update(window.app_handle(), &dashboard.accounts, visible);
}

#[cfg(test)]
mod tests {
    use super::{choose_negative_direction, idle_anchor};
    use tauri::{PhysicalPosition, PhysicalSize};

    #[test]
    fn expanded_left_window_saves_the_idle_orb_anchor() {
        let anchor = idle_anchor(
            PhysicalPosition::new(100, 200),
            PhysicalSize::new(636, 360),
            1.5,
            "expanded",
            "left",
            "up",
        );
        assert_eq!(anchor, PhysicalPosition::new(604, 428));
    }

    #[test]
    fn idle_window_saves_its_top_left_position() {
        let anchor = idle_anchor(
            PhysicalPosition::new(320, 240),
            PhysicalSize::new(120, 120),
            1.5,
            "idle",
            "right",
            "down",
        );
        assert_eq!(anchor, PhysicalPosition::new(320, 240));
    }

    #[test]
    fn placement_flips_only_when_the_preferred_side_no_longer_fits() {
        assert!(!choose_negative_direction(100, 0, 1000, 424, 44, 12, false));
        assert!(choose_negative_direction(900, 0, 1000, 424, 44, 12, false));
    }

    #[test]
    fn placement_uses_the_requested_priority_when_both_sides_fit() {
        assert!(choose_negative_direction(500, 0, 1000, 424, 44, 12, true));
        assert!(!choose_negative_direction(500, 0, 1000, 424, 44, 12, false));
    }
}
