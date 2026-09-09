use crate::models::UserSettings;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{PhysicalPosition, WebviewWindow};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "CodexAccountManager";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    #[serde(flatten)]
    user: UserSettings,
    positions: Vec<SavedPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedPosition {
    monitor_name: String,
    scale_factor: f64,
    x: i32,
    y: i32,
}

pub fn load_user_settings() -> Result<UserSettings, String> {
    Ok(load()?.user)
}

pub fn update_user_settings(mut settings: UserSettings) -> Result<UserSettings, String> {
    normalize(&mut settings);
    set_launch_at_login(settings.launch_at_login)?;
    let mut stored = load()?;
    stored.user = settings.clone();
    save(&stored)?;
    Ok(settings)
}

fn normalize(settings: &mut UserSettings) {
    settings.refresh_interval_minutes = settings.refresh_interval_minutes.clamp(1, 60);
}

pub fn save_window_position(window: &WebviewWindow) -> Result<(), String> {
    let mut stored = load()?;
    if !stored.user.remember_position {
        return Ok(());
    }
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or("无法确定当前显示器")?;
    let monitor_name = monitor.name().cloned().unwrap_or_default();
    let scale_factor = monitor.scale_factor();
    stored.positions.retain(|item| {
        item.monitor_name != monitor_name || (item.scale_factor - scale_factor).abs() >= 0.01
    });
    stored.positions.push(SavedPosition {
        monitor_name,
        scale_factor,
        x: position.x,
        y: position.y,
    });
    save(&stored)
}

pub fn restore_window_position(window: &WebviewWindow) -> Result<(), String> {
    let stored = load()?;
    if !stored.user.remember_position || stored.positions.is_empty() {
        return Ok(());
    }
    let monitors = window
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let current_scale = window.scale_factor().unwrap_or(1.0);
    let saved = stored
        .positions
        .iter()
        .min_by(|left, right| {
            (left.scale_factor - current_scale)
                .abs()
                .total_cmp(&(right.scale_factor - current_scale).abs())
        })
        .ok_or("没有已保存的悬浮球位置")?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            monitor
                .name()
                .is_some_and(|name| name == &saved.monitor_name)
        })
        .or_else(|| {
            monitors.iter().find(|monitor| {
                let area = monitor.work_area();
                saved.x >= area.position.x
                    && saved.x < area.position.x + area.size.width as i32
                    && saved.y >= area.position.y
                    && saved.y < area.position.y + area.size.height as i32
            })
        })
        .or_else(|| monitors.first())
        .ok_or("没有可用显示器")?;
    let area = monitor.work_area();
    let width = (96.0 * monitor.scale_factor()).round() as i32;
    let height = (96.0 * monitor.scale_factor()).round() as i32;
    let x = saved.x.clamp(
        area.position.x,
        area.position.x + area.size.width as i32 - width,
    );
    let y = saved.y.clamp(
        area.position.y,
        area.position.y + area.size.height as i32 - height,
    );
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

fn load() -> Result<StoredSettings, String> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(StoredSettings::default());
    }
    serde_json::from_slice(&fs::read(path).map_err(|error| format!("读取设置失败：{error}"))?)
        .map_err(|error| format!("设置文件格式错误：{error}"))
}

fn save(settings: &StoredSettings) -> Result<(), String> {
    let path = settings_path()?;
    let parent = path.parent().ok_or("设置文件路径无父目录")?;
    fs::create_dir_all(parent).map_err(|error| format!("创建设置目录失败：{error}"))?;
    let content =
        serde_json::to_vec_pretty(settings).map_err(|error| format!("序列化设置失败：{error}"))?;
    fs::write(path, content).map_err(|error| format!("保存设置失败：{error}"))
}

fn settings_path() -> Result<PathBuf, String> {
    crate::vault::app_data_dir().map(|root| root.join("settings.json"))
}

fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = current_user
        .create_subkey(RUN_KEY)
        .map_err(|error| format!("打开 Windows 启动项失败：{error}"))?;
    if enabled {
        let executable =
            std::env::current_exe().map_err(|error| format!("读取程序路径失败：{error}"))?;
        run.set_value(RUN_VALUE, &format!("\"{}\"", executable.display()))
            .map_err(|error| format!("写入开机启动项失败：{error}"))
    } else {
        match run.delete_value(RUN_VALUE) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("移除开机启动项失败：{error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::models::UserSettings;

    #[test]
    fn refresh_interval_is_kept_in_safe_range() {
        let mut low = UserSettings {
            refresh_interval_minutes: 0,
            ..UserSettings::default()
        };
        normalize(&mut low);
        assert_eq!(low.refresh_interval_minutes, 1);

        let mut high = UserSettings {
            refresh_interval_minutes: 120,
            ..UserSettings::default()
        };
        normalize(&mut high);
        assert_eq!(high.refresh_interval_minutes, 60);
    }
}
