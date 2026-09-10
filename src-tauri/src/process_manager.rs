use std::{
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[cfg(not(debug_assertions))]
use std::collections::HashMap;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(not(debug_assertions))]
pub fn relaunch_outside_codex_tree() -> Result<bool, String> {
    if !has_codex_desktop_ancestor()? {
        return Ok(false);
    }

    let executable =
        std::env::current_exe().map_err(|error| format!("无法读取管理器程序路径：{error}"))?;
    Command::new("explorer.exe")
        .arg(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("无法从 Windows 桌面重新启动管理器：{error}"))?;
    Ok(true)
}

#[cfg(not(debug_assertions))]
fn has_codex_desktop_ancestor() -> Result<bool, String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
    };

    // SAFETY: the snapshot handle is checked before use, PROCESSENTRY32W has
    // the required size set, and the handle is closed on every successful path.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(format!(
            "无法读取 Windows 进程列表：{}",
            std::io::Error::last_os_error()
        ));
    }

    let mut processes = HashMap::new();
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
        loop {
            let name_end = entry
                .szExeFile
                .iter()
                .position(|character| *character == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..name_end]);
            processes.insert(entry.th32ProcessID, (entry.th32ParentProcessID, name));
            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    unsafe { CloseHandle(snapshot) };

    Ok(has_named_ancestor(
        std::process::id(),
        &processes,
        "ChatGPT.exe",
    ))
}

#[cfg(not(debug_assertions))]
fn has_named_ancestor(
    current_pid: u32,
    processes: &HashMap<u32, (u32, String)>,
    expected_name: &str,
) -> bool {
    let mut pid = current_pid;
    for _ in 0..16 {
        let Some((parent_pid, _)) = processes.get(&pid) else {
            return false;
        };
        if *parent_pid == 0 || *parent_pid == pid {
            return false;
        }
        let Some((_, parent_name)) = processes.get(parent_pid) else {
            return false;
        };
        if parent_name.eq_ignore_ascii_case(expected_name) {
            return true;
        }
        pid = *parent_pid;
    }
    false
}

pub fn is_codex_running() -> bool {
    codex_pids().is_ok_and(|pids| !pids.is_empty())
}

pub fn is_codex_cli_running() -> Result<bool, String> {
    let script = r#"
$all = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue
$items = $all | Where-Object { $_.Name -eq 'codex.exe' -and $_.CommandLine -notlike '*app-server*' }
foreach ($item in $items) {
  $cursor = $item
  $ownedByDesktop = $false
  for ($depth = 0; $depth -lt 8 -and $cursor; $depth++) {
    if ($cursor.Name -eq 'ChatGPT.exe' -and $cursor.ExecutablePath -and ($cursor.ExecutablePath -like '*\OpenAI.Codex_*' -or $cursor.ExecutablePath -like '*\OpenAI\Codex\*')) {
      $ownedByDesktop = $true
      break
    }
    $parentId = $cursor.ParentProcessId
    $cursor = $all | Where-Object { $_.ProcessId -eq $parentId } | Select-Object -First 1
  }
  if (-not $ownedByDesktop) { $item.ProcessId }
}
"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("无法检查 Codex CLI 进程：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim().parse::<u32>().is_ok()))
}

pub fn stop_codex_desktop() -> Result<bool, String> {
    let pids = codex_pids()?;
    if pids.is_empty() {
        return Ok(false);
    }
    let id_list = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    run_powershell(&format!(
        "$ids=@({id_list}); foreach($id in $ids){{ $p=Get-Process -Id $id -ErrorAction SilentlyContinue; if($p){{ [void]$p.CloseMainWindow() }} }}"
    ))?;

    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if codex_pids()?.is_empty() {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(400));
    }

    let remaining = codex_pids()?;
    if !remaining.is_empty() {
        let remaining_ids = remaining
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        run_powershell(&format!(
            "$ids=@({remaining_ids}); Stop-Process -Id $ids -Force -ErrorAction SilentlyContinue"
        ))?;
    }
    Ok(true)
}

pub fn start_codex_desktop() -> Result<(), String> {
    run_powershell(
        "$app=Get-StartApps | Where-Object { $_.Name -eq 'Codex' -or $_.AppID -like 'OpenAI.Codex*!App' } | Select-Object -First 1; if(-not $app){ throw 'Codex app not found' }; Start-Process ('shell:AppsFolder\\' + $app.AppID)",
    )
}

fn codex_pids() -> Result<Vec<u32>, String> {
    let script = "$items=Get-CimInstance Win32_Process -Filter \"Name='ChatGPT.exe'\" -ErrorAction SilentlyContinue | Where-Object { $_.ExecutablePath -and ($_.ExecutablePath -like '*\\OpenAI.Codex_*' -or $_.ExecutablePath -like '*\\OpenAI\\Codex\\*') }; $items.ProcessId";
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("无法检查 Codex 进程：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect())
}

fn run_powershell(script: &str) -> Result<(), String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("PowerShell 执行失败：{error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "Windows 系统操作失败".to_string()
        } else {
            message
        })
    }
}
