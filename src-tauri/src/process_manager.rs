use std::{
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_codex_running() -> bool {
    codex_pids().is_ok_and(|pids| !pids.is_empty())
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
