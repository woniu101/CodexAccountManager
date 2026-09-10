use crate::models::ManagedAccount;
use tauri::{
    AppHandle, Runtime,
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
};

pub const TRAY_ID: &str = "main-tray";
pub const SWITCH_ACCOUNT_PREFIX: &str = "switch-account:";

pub fn build<R: Runtime>(
    app: &AppHandle<R>,
    accounts: &[ManagedAccount],
    window_visible: bool,
) -> tauri::Result<Menu<R>> {
    let title = MenuItem::with_id(
        app,
        "tray-title",
        "Codex Account Manager",
        false,
        None::<&str>,
    )?;
    let current = accounts.iter().find(|account| account.is_active);
    let account_status = MenuItem::with_id(
        app,
        "tray-current",
        current_account_text(current),
        false,
        None::<&str>,
    )?;
    let quota_status =
        MenuItem::with_id(app, "tray-quota", quota_text(current), false, None::<&str>)?;

    let toggle = MenuItem::with_id(
        app,
        "toggle-visibility",
        if window_visible {
            "隐藏悬浮球"
        } else {
            "显示悬浮球"
        },
        true,
        None::<&str>,
    )?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;

    let account_items = accounts
        .iter()
        .map(|account| {
            CheckMenuItem::with_id(
                app,
                format!("{SWITCH_ACCOUNT_PREFIX}{}", account.id),
                account_menu_text(account),
                !account.is_active,
                account.is_active,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let account_item_refs = account_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<R>)
        .collect::<Vec<_>>();
    let switch_accounts = Submenu::with_id_and_items(
        app,
        "switch-accounts",
        "切换账号",
        !account_items.is_empty(),
        &account_item_refs,
    )?;

    let add = MenuItem::with_id(app, "add", "添加账号…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator_1 = PredefinedMenuItem::separator(app)?;
    let separator_2 = PredefinedMenuItem::separator(app)?;
    let separator_3 = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &title,
            &account_status,
            &quota_status,
            &separator_1,
            &toggle,
            &refresh,
            &separator_2,
            &switch_accounts,
            &add,
            &settings,
            &separator_3,
            &quit,
        ],
    )
}

pub fn update<R: Runtime>(
    app: &AppHandle<R>,
    accounts: &[ManagedAccount],
    window_visible: bool,
) -> Result<(), String> {
    let menu = build(app, accounts, window_visible).map_err(|error| error.to_string())?;
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "找不到系统托盘图标".to_string())?;
    tray.set_menu(Some(menu)).map_err(|error| error.to_string())
}

fn current_account_text(account: Option<&ManagedAccount>) -> String {
    let Some(account) = account else {
        return "当前：未检测到账号".to_string();
    };
    let plan = account
        .plan_type
        .as_deref()
        .filter(|plan| !plan.is_empty())
        .map(|plan| format!(" · {}", plan.to_uppercase()))
        .unwrap_or_default();
    format!("当前：{}{plan}", account_name(account))
}

fn quota_text(account: Option<&ManagedAccount>) -> String {
    let Some(account) = account else {
        return "额度：暂无数据".to_string();
    };
    let five_hour = account
        .five_hour
        .as_ref()
        .map(|quota| format!("{}%", quota.remaining_percent.round() as i32))
        .unwrap_or_else(|| "--".to_string());
    let weekly = account
        .weekly
        .as_ref()
        .map(|quota| format!("{}%", quota.remaining_percent.round() as i32))
        .unwrap_or_else(|| "--".to_string());
    format!("额度：5 小时 {five_hour} · 本周 {weekly}")
}

fn account_menu_text(account: &ManagedAccount) -> String {
    let plan = account
        .plan_type
        .as_deref()
        .filter(|plan| !plan.is_empty())
        .map(|plan| format!(" · {}", plan.to_uppercase()))
        .unwrap_or_default();
    format!("{}{plan}", account_name(account))
}

fn account_name(account: &ManagedAccount) -> String {
    account
        .alias
        .as_deref()
        .filter(|alias| !alias.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| mask_email(&account.email))
}

fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return email.to_string();
    };
    let visible = local.chars().take(2).collect::<String>();
    if local.chars().count() <= 2 {
        format!("{visible}@{domain}")
    } else {
        format!("{visible}…@{domain}")
    }
}

#[cfg(test)]
mod tests {
    use super::mask_email;

    #[test]
    fn tray_email_is_masked_but_still_identifiable() {
        assert_eq!(mask_email("brave179437721@gmail.com"), "br…@gmail.com");
        assert_eq!(mask_email("ab@example.com"), "ab@example.com");
    }
}
