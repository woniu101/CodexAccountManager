use crate::models::{ManagedAccount, QuotaWindow};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant, SystemTime},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct AppServerSession {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    notifications: Vec<Value>,
    next_id: u64,
    pub home: PathBuf,
}

impl AppServerSession {
    pub fn start(home: PathBuf) -> Result<Self, String> {
        let executable = find_codex_executable()?;
        let mut child = Command::new(executable)
            .arg("app-server")
            .arg("--listen")
            .arg("stdio://")
            .env("CODEX_HOME", &home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("无法启动 Codex App Server：{error}"))?;

        let stdin = child.stdin.take().ok_or("无法连接 App Server 标准输入")?;
        let stdout = child.stdout.take().ok_or("无法连接 App Server 标准输出")?;
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && sender.send(value).is_err()
                {
                    break;
                }
            }
        });

        let mut session = Self {
            child,
            stdin,
            messages: receiver,
            notifications: Vec::new(),
            next_id: 1,
            home,
        };

        session.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codex-account-manager",
                    "title": "Codex Account Manager",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {}
            }),
            Duration::from_secs(12),
        )?;
        session.notify("initialized", json!({}))?;
        Ok(session)
    }

    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_message(&json!({ "method": method, "id": id, "params": params }))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("App Server 请求超时：{method}"));
            }
            let message = self
                .messages
                .recv_timeout(remaining)
                .map_err(|_| format!("App Server 请求超时或连接中断：{method}"))?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(format!("App Server 返回错误：{error}"));
                }
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }
            self.notifications.push(message);
        }
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.write_message(&json!({ "method": method, "params": params }))
    }

    pub fn take_notification(&mut self, method: &str) -> Option<Value> {
        if let Some(index) = self
            .notifications
            .iter()
            .position(|item| item.get("method").and_then(Value::as_str) == Some(method))
        {
            return Some(self.notifications.remove(index));
        }
        while let Ok(message) = self.messages.try_recv() {
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return Some(message);
            }
            self.notifications.push(message);
        }
        None
    }

    pub fn query_account(&mut self) -> Result<ManagedAccount, String> {
        let account_result = self.request(
            "account/read",
            json!({ "refreshToken": true }),
            Duration::from_secs(12),
        )?;
        let account = account_result
            .get("account")
            .filter(|value| !value.is_null())
            .ok_or("Codex 当前未登录 ChatGPT 账号")?;
        if account.get("type").and_then(Value::as_str) != Some("chatgpt") {
            return Err("当前 Codex 登录方式不是 ChatGPT 账号".to_string());
        }

        let rate_result = self.request(
            "account/rateLimits/read",
            json!({}),
            Duration::from_secs(12),
        )?;
        let email = account
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or("未知账号")
            .to_string();
        let plan_type = account
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_string);
        let account_id = rate_result
            .get("accountId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| read_account_id(&self.home))
            .unwrap_or_else(|| format!("account-{}", sanitize_id(&email)));

        let pool = rate_result
            .get("rateLimitsByLimitId")
            .and_then(|value| value.get("codex"))
            .or_else(|| rate_result.get("rateLimits"));
        let mut windows = Vec::new();
        if let Some(pool) = pool {
            for field in ["primary", "secondary"] {
                if let Some(window) = pool.get(field).filter(|value| !value.is_null())
                    && let Some(parsed) = parse_quota_window(pool, window)
                {
                    windows.push(parsed);
                }
            }
        }

        let five_hour = windows
            .iter()
            .find(|window| window.window_duration_mins == 300)
            .cloned();
        let weekly = windows
            .iter()
            .find(|window| window.window_duration_mins == 10_080)
            .cloned();
        let extra_limits = windows
            .into_iter()
            .filter(|window| !matches!(window.window_duration_mins, 300 | 10_080))
            .collect();

        Ok(ManagedAccount {
            id: account_id,
            email,
            alias: None,
            plan_type,
            is_active: false,
            credential_state: "valid".to_string(),
            five_hour,
            weekly,
            extra_limits,
            last_updated_at: Some(unix_now()),
            last_error: None,
        })
    }

    fn write_message(&mut self, message: &Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.stdin, message)
            .map_err(|error| format!("JSON-RPC 序列化失败：{error}"))?;
        self.stdin
            .write_all(b"\n")
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("写入 App Server 失败：{error}"))
    }
}

impl Drop for AppServerSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn query_home(home: PathBuf) -> Result<ManagedAccount, String> {
    let first_error = match AppServerSession::start(home.clone())
        .and_then(|mut session| session.query_account())
    {
        Ok(account) => return Ok(account),
        Err(error) => error,
    };
    thread::sleep(Duration::from_millis(180));
    AppServerSession::start(home)
        .and_then(|mut session| session.query_account())
        .map_err(|second_error| format!("{second_error}（首次尝试：{first_error}）"))
}

pub fn current_codex_home() -> Result<PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|path| path.join(".codex"))
        .ok_or_else(|| "无法确定当前 Windows 用户目录".to_string())
}

pub fn find_codex_executable() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEX_EXECUTABLE").map(PathBuf::from)
        && path.is_file()
    {
        return Ok(path);
    }

    let mut candidates = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        collect_codex_executables(
            &local.join("OpenAI").join("Codex").join("bin"),
            0,
            &mut candidates,
        );
    }
    if let Ok(output) = Command::new("where.exe")
        .arg("codex.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let path = PathBuf::from(line.trim());
            if path.is_file() {
                candidates.push(path);
            }
        }
    }

    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    candidates
        .pop()
        .ok_or_else(|| "没有找到 Codex 可执行文件，请先安装或更新 Codex Desktop".to_string())
}

fn collect_codex_executables(root: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth > 3 || !root.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_codex_executables(&path, depth + 1, output);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("codex.exe"))
        {
            output.push(path);
        }
    }
}

fn parse_quota_window(pool: &Value, window: &Value) -> Option<QuotaWindow> {
    let used_percent = window.get("usedPercent")?.as_f64()?.clamp(0.0, 100.0);
    Some(QuotaWindow {
        limit_id: pool
            .get("limitId")
            .and_then(Value::as_str)
            .unwrap_or("codex")
            .to_string(),
        used_percent,
        remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
        window_duration_mins: window.get("windowDurationMins")?.as_u64()?,
        resets_at: window.get("resetsAt").and_then(Value::as_i64),
    })
}

fn read_account_id(home: &Path) -> Option<String> {
    let value: Value = serde_json::from_slice(&fs::read(home.join("auth.json")).ok()?).ok()?;
    value
        .pointer("/tokens/account_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::{parse_quota_window, sanitize_id};
    use serde_json::json;

    #[test]
    fn quota_window_converts_used_to_remaining() {
        let pool = json!({ "limitId": "codex" });
        let window = json!({
            "usedPercent": 37.5,
            "windowDurationMins": 300,
            "resetsAt": 1_800_000_000
        });

        let parsed = parse_quota_window(&pool, &window).expect("valid quota window");
        assert_eq!(parsed.limit_id, "codex");
        assert_eq!(parsed.used_percent, 37.5);
        assert_eq!(parsed.remaining_percent, 62.5);
        assert_eq!(parsed.window_duration_mins, 300);
    }

    #[test]
    fn quota_percent_is_clamped() {
        let pool = json!({});
        let window = json!({ "usedPercent": 120.0, "windowDurationMins": 10_080 });
        let parsed = parse_quota_window(&pool, &window).expect("valid quota window");
        assert_eq!(parsed.used_percent, 100.0);
        assert_eq!(parsed.remaining_percent, 0.0);
    }

    #[test]
    fn account_id_is_safe_for_directory_names() {
        assert_eq!(
            sanitize_id("plus/user@example.com"),
            "plus_user_example_com"
        );
    }
}
