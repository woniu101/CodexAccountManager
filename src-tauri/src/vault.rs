use crate::{
    app_server::{current_codex_home, query_home, sanitize_id},
    models::ManagedAccount,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::c_void,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMeta {
    pub id: String,
    pub email: String,
    pub alias: Option<String>,
    pub plan_type: Option<String>,
    pub credential_file: String,
    pub cached: Option<ManagedAccount>,
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("CodexAccountManager"))
        .ok_or_else(|| "无法确定 LOCALAPPDATA 目录".to_string())
}

pub fn auth_path() -> Result<PathBuf, String> {
    Ok(current_codex_home()?.join("auth.json"))
}

pub fn load_accounts() -> Result<Vec<AccountMeta>, String> {
    let path = app_data_dir()?.join("accounts.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(display_io("读取账号索引失败"))?)
        .map_err(|error| format!("账号索引格式错误：{error}"))
}

pub fn has_account(account_id: &str) -> Result<bool, String> {
    Ok(load_accounts()?.iter().any(|item| item.id == account_id))
}

pub fn save_accounts(accounts: &[AccountMeta]) -> Result<(), String> {
    let root = app_data_dir()?;
    fs::create_dir_all(&root).map_err(display_io("创建应用数据目录失败"))?;
    let final_path = root.join("accounts.json");
    let temporary = root.join("accounts.json.new");
    let content = serde_json::to_vec_pretty(accounts)
        .map_err(|error| format!("账号索引序列化失败：{error}"))?;
    write_and_flush(&temporary, &content)?;
    atomic_replace(&temporary, &final_path)
}

pub fn import_current_account() -> Result<ManagedAccount, String> {
    let home = current_codex_home()?;
    let auth = fs::read(home.join("auth.json")).map_err(display_io("读取当前 Codex 登录态失败"))?;
    let mut account = query_home(home)?;
    account.is_active = true;
    store_account(&account, &auth)?;
    Ok(account)
}

pub fn store_account(account: &ManagedAccount, auth: &[u8]) -> Result<(), String> {
    let root = app_data_dir()?;
    let folder_name = sanitize_id(&account.id);
    let account_dir = root.join("accounts").join(&folder_name);
    fs::create_dir_all(&account_dir).map_err(display_io("创建账号目录失败"))?;
    let encrypted = protect(auth)?;
    let credential_file = format!("accounts/{folder_name}/credential.bin");
    write_and_flush(&root.join(&credential_file), &encrypted)?;

    let mut accounts = load_accounts()?;
    if let Some(existing) = accounts.iter_mut().find(|item| item.id == account.id) {
        existing.email.clone_from(&account.email);
        existing.plan_type.clone_from(&account.plan_type);
        existing.cached = Some(account.clone());
        existing.credential_file = credential_file;
    } else {
        accounts.push(AccountMeta {
            id: account.id.clone(),
            email: account.email.clone(),
            alias: account.alias.clone(),
            plan_type: account.plan_type.clone(),
            credential_file,
            cached: Some(account.clone()),
        });
    }
    save_accounts(&accounts)
}

pub fn ensure_current_imported() -> Result<(), String> {
    if load_accounts()?.is_empty() && auth_path()?.exists() {
        import_current_account()?;
    }
    Ok(())
}

pub fn refresh_all_accounts() -> Result<Vec<ManagedAccount>, String> {
    ensure_current_imported()?;
    let mut metadata = load_accounts()?;
    let live_id = read_account_id(&auth_path()?).ok();
    let mut views = Vec::new();

    for meta in &mut metadata {
        let is_active = live_id.as_deref() == Some(meta.id.as_str());
        let result = if is_active {
            query_home(current_codex_home()?)
        } else {
            query_inactive(meta)
        };

        match result {
            Ok(mut account) => {
                account.is_active = is_active;
                account.alias.clone_from(&meta.alias);
                meta.email.clone_from(&account.email);
                meta.plan_type.clone_from(&account.plan_type);
                meta.cached = Some(account.clone());
                views.push(account);
            }
            Err(message) => {
                let mut cached = meta.cached.clone().unwrap_or_else(|| ManagedAccount {
                    id: meta.id.clone(),
                    email: meta.email.clone(),
                    alias: meta.alias.clone(),
                    plan_type: meta.plan_type.clone(),
                    credential_state: "expired".to_string(),
                    ..ManagedAccount::default()
                });
                cached.is_active = is_active;
                let credential_path = app_data_dir()?.join(&meta.credential_file);
                if !credential_path.exists() {
                    cached.credential_state = "missing".to_string();
                } else if message.contains("未登录") || message.contains("认证") {
                    cached.credential_state = "expired".to_string();
                }
                cached.last_error = Some(message);
                views.push(cached);
            }
        }
    }
    save_accounts(&metadata)?;
    views.sort_by_key(|account| !account.is_active);
    Ok(views)
}

pub fn target_auth(account_id: &str) -> Result<Vec<u8>, String> {
    let accounts = load_accounts()?;
    let meta = accounts
        .iter()
        .find(|item| item.id == account_id)
        .ok_or_else(|| "目标账号不存在".to_string())?;
    let encrypted = fs::read(app_data_dir()?.join(&meta.credential_file))
        .map_err(display_io("读取目标账号凭据失败"))?;
    unprotect(&encrypted)
}

pub fn save_live_auth_to_vault() -> Result<Option<String>, String> {
    let path = auth_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let account_id = read_account_id(&path)?;
    let auth = fs::read(&path).map_err(display_io("读取当前登录态失败"))?;
    let mut accounts = load_accounts()?;
    if let Some(meta) = accounts.iter_mut().find(|item| item.id == account_id) {
        let encrypted = protect(&auth)?;
        write_and_flush(&app_data_dir()?.join(&meta.credential_file), &encrypted)?;
    }
    Ok(Some(account_id))
}

pub fn replace_live_auth(content: &[u8]) -> Result<(), String> {
    let final_path = auth_path()?;
    let parent = final_path.parent().ok_or("登录态路径无父目录")?;
    fs::create_dir_all(parent).map_err(display_io("创建 Codex 目录失败"))?;
    let temporary = parent.join("auth.json.switching");
    write_and_flush(&temporary, content)?;
    atomic_replace(&temporary, &final_path)
}

pub fn write_switch_journal(source: Option<&str>, target: &str, phase: &str) -> Result<(), String> {
    let root = app_data_dir()?;
    fs::create_dir_all(&root).map_err(display_io("创建应用数据目录失败"))?;
    let journal = serde_json::json!({
        "sourceAccountId": source,
        "targetAccountId": target,
        "phase": phase,
        "updatedAt": SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });
    write_and_flush(
        &root.join("switch-journal.json"),
        serde_json::to_string_pretty(&journal)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )
}

pub fn recover_incomplete_switch() -> Result<(), String> {
    let path = app_data_dir()?.join("switch-journal.json");
    if !path.exists() {
        return Ok(());
    }
    let journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).map_err(display_io("读取切换恢复日志失败"))?)
            .map_err(|error| format!("切换恢复日志格式错误：{error}"))?;
    let phase = journal
        .get("phase")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if phase == "prepared" {
        clear_switch_journal();
        return Ok(());
    }
    let target = journal
        .get("targetAccountId")
        .and_then(serde_json::Value::as_str);
    let current = read_account_id(&auth_path()?).ok();
    if current.as_deref() == target {
        clear_switch_journal();
        return Ok(());
    }
    if let Some(source) = journal
        .get("sourceAccountId")
        .and_then(serde_json::Value::as_str)
    {
        replace_live_auth(&target_auth(source)?)?;
    }
    clear_switch_journal();
    Ok(())
}

pub fn has_switch_journal() -> bool {
    app_data_dir().is_ok_and(|root| root.join("switch-journal.json").exists())
}

pub fn cleanup_stale_temp_dirs() {
    let Ok(temp_root) = app_data_dir().map(|root| root.join("temp")) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&temp_root) else {
        return;
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let is_stale = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified < cutoff);
        if is_stale {
            let _ = fs::remove_dir_all(path);
        }
    }
}

pub fn clear_switch_journal() {
    if let Ok(root) = app_data_dir() {
        let _ = fs::remove_file(root.join("switch-journal.json"));
    }
}

fn query_inactive(meta: &mut AccountMeta) -> Result<ManagedAccount, String> {
    let root = app_data_dir()?;
    let encrypted =
        fs::read(root.join(&meta.credential_file)).map_err(display_io("读取账号凭据失败"))?;
    let auth = unprotect(&encrypted)?;
    let temporary = root.join("temp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&temporary).map_err(display_io("创建账号临时目录失败"))?;
    write_and_flush(&temporary.join("auth.json"), &auth)?;
    write_and_flush(
        &temporary.join("config.toml"),
        b"cli_auth_credentials_store = \"file\"\n",
    )?;

    let result = query_home(temporary.clone());
    if result.is_ok()
        && let Ok(updated) = fs::read(temporary.join("auth.json"))
    {
        let encrypted = protect(&updated)?;
        write_and_flush(&root.join(&meta.credential_file), &encrypted)?;
    }
    let _ = fs::remove_dir_all(&temporary);
    result
}

fn read_account_id(path: &Path) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(display_io("读取登录态失败"))?)
            .map_err(|error| format!("登录态格式错误：{error}"))?;
    value
        .pointer("/tokens/account_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "登录态中缺少 account_id".to_string())
}

fn write_and_flush(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(display_io("创建目录失败"))?;
    }
    let mut file = File::create(path).map_err(display_io("创建文件失败"))?;
    file.write_all(content)
        .map_err(display_io("写入文件失败"))?;
    file.sync_all().map_err(display_io("刷新文件失败"))
}

fn display_io(prefix: &'static str) -> impl Fn(std::io::Error) -> String {
    move |error| format!("{prefix}：{error}")
}

#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    size: u32,
    data: *mut u8,
}

#[cfg(windows)]
#[link(name = "Crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        input: *mut DataBlob,
        description: *const u16,
        entropy: *mut DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut DataBlob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *mut DataBlob,
        description: *mut *mut u16,
        entropy: *mut DataBlob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut DataBlob,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
    fn MoveFileExW(existing: *const u16, destination: *const u16, flags: u32) -> i32;
}

#[cfg(windows)]
fn protect(content: &[u8]) -> Result<Vec<u8>, String> {
    crypt(content, true)
}

#[cfg(windows)]
fn unprotect(content: &[u8]) -> Result<Vec<u8>, String> {
    crypt(content, false)
}

#[cfg(windows)]
fn crypt(content: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    let mut input = DataBlob {
        size: content.len() as u32,
        data: content.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        size: 0,
        data: std::ptr::null_mut(),
    };
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &mut input,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &mut input,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err("Windows DPAPI 操作失败".to_string());
    }
    let result = unsafe { std::slice::from_raw_parts(output.data, output.size as usize).to_vec() };
    unsafe {
        LocalFree(output.data.cast());
    }
    Ok(result)
}

#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前版本仅支持 Windows".to_string())
}

#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前版本仅支持 Windows".to_string())
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) };
    if result == 0 {
        Err(format!(
            "原子替换文件失败：{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{protect, unprotect};

    #[test]
    fn dpapi_round_trip_uses_current_windows_user() {
        let original = b"codex-account-manager-test";
        let encrypted = protect(original).expect("DPAPI encryption should succeed");
        assert_ne!(encrypted, original);
        assert_eq!(
            unprotect(&encrypted).expect("DPAPI decryption should succeed"),
            original
        );
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(display_io("替换文件失败"))
}
