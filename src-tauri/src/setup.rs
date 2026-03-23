use serde::Serialize;
use std::process::{Command as StdCmd, Stdio};
use std::path::PathBuf;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCmd;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct EnvironmentStatus {
    #[serde(rename = "nodeInstalled")]
    pub node_installed: bool,
    #[serde(rename = "nodeVersion")]
    pub node_version: Option<String>,
    #[serde(rename = "openclawInstalled")]
    pub openclaw_installed: bool,
    #[serde(rename = "openclawVersion")]
    pub openclaw_version: Option<String>,
    #[serde(rename = "openclawPath")]
    pub openclaw_path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstallProgressEvent {
    pub line: String,
    #[serde(rename = "type")]
    pub kind: String, // "command" | "stdout" | "stderr" | "success" | "error"
}

// ── OS helpers ────────────────────────────────────────────────────────────────

/// Run a command and return its trimmed stdout, or None if it fails.
fn probe(prog: &str, args: &[&str]) -> Option<String> {
    // On Windows, wrap in "cmd /c" so .cmd shims are resolved
    #[cfg(target_os = "windows")]
    let out = StdCmd::new("cmd")
        .arg("/c")
        .arg(prog)
        .args(args)
        .output()
        .ok()?;

    #[cfg(not(target_os = "windows"))]
    let out = StdCmd::new(prog).args(args).output().ok()?;

    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            // Some tools write to stderr
            let se = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !se.is_empty() { return Some(se); }
        }
        Some(s)
    } else {
        None
    }
}

/// Build the appropriate npm command for the current OS.
#[cfg(target_os = "windows")]
fn npm_cmd() -> TokioCmd {
    let mut c = TokioCmd::new("cmd");
    c.arg("/c").arg("npm");
    c
}

#[cfg(not(target_os = "windows"))]
fn npm_cmd() -> TokioCmd {
    TokioCmd::new("npm")
}

// ── Path detection ────────────────────────────────────────────────────────────

/// Returns the path of the openclaw binary if resolvable.
pub fn detect_openclaw_path() -> Option<String> {
    // 1. System PATH lookup – prefer .cmd shim on Windows
    #[cfg(target_os = "windows")]
    if let Ok(out) = StdCmd::new("cmd").args(["/c", "where", "openclaw"]).output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // `where` may return multiple lines; prefer the .cmd entry
            let cmd_line = stdout.lines()
                .find(|l| l.to_lowercase().ends_with(".cmd"))
                .or_else(|| stdout.lines().next());
            if let Some(p) = cmd_line {
                let p = p.trim().to_string();
                if !p.is_empty() { return Some(p); }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    if let Ok(out) = StdCmd::new("which").arg("openclaw").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() { return Some(p); }
        }
    }

    // 2. npm global bin directory
    #[cfg(target_os = "windows")]
    let npm_bin_out = StdCmd::new("cmd").args(["/c", "npm", "bin", "-g"]).output().ok();
    #[cfg(not(target_os = "windows"))]
    let npm_bin_out = StdCmd::new("npm").args(["bin", "-g"]).output().ok();

    if let Some(out) = npm_bin_out {
        if out.status.success() {
            let bin_dir = String::from_utf8_lossy(&out.stdout).trim().to_string();

            #[cfg(target_os = "windows")]
            let candidate = PathBuf::from(&bin_dir).join("openclaw.cmd");
            #[cfg(not(target_os = "windows"))]
            let candidate = PathBuf::from(&bin_dir).join("openclaw");

            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // 3. Fallback – rely on PATH at runtime
    Some("openclaw".to_string())
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn check_environment() -> EnvironmentStatus {
    let node_version = probe("node", &["--version"]);
    let openclaw_version = probe("openclaw", &["--version"]);
    let openclaw_path = if openclaw_version.is_some() {
        detect_openclaw_path()
    } else {
        None
    };

    EnvironmentStatus {
        node_installed: node_version.is_some(),
        node_version,
        openclaw_installed: openclaw_version.is_some(),
        openclaw_version,
        openclaw_path,
    }
}

#[tauri::command]
pub async fn install_openclaw(app: tauri::AppHandle) -> Result<String, String> {
    let emit = |line: &str, kind: &str| {
        let _ = app.emit("install_progress", InstallProgressEvent {
            line: line.to_string(),
            kind: kind.to_string(),
        });
    };

    emit("▸ npm install -g openclaw", "command");

    let mut child = npm_cmd()
        .args(["install", "-g", "openclaw"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("npm konnte nicht gestartet werden: {}", e))?;

    // Stream stdout in background task
    if let Some(stdout) = child.stdout.take() {
        let a = app.clone();
        let mut lines = BufReader::new(stdout).lines();
        tauri::async_runtime::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = a.emit("install_progress", InstallProgressEvent {
                    line,
                    kind: "stdout".into(),
                });
            }
        });
    }

    // Stream stderr in background task
    if let Some(stderr) = child.stderr.take() {
        let a = app.clone();
        let mut lines = BufReader::new(stderr).lines();
        tauri::async_runtime::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = a.emit("install_progress", InstallProgressEvent {
                    line,
                    kind: "stderr".into(),
                });
            }
        });
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        emit(
            &format!("✗ Installation fehlgeschlagen (exit code {})", code),
            "error",
        );
        return Err(format!("npm install fehlgeschlagen (exit {})", code));
    }

    let path = detect_openclaw_path().unwrap_or_else(|| "openclaw".to_string());
    emit(&format!("✓ openclaw installiert: {}", path), "success");
    Ok(path)
}

#[tauri::command]
pub fn validate_openclaw_path(path: String) -> Result<String, String> {
    let version = probe(&path, &["--version"])
        .ok_or_else(|| format!("'{}' antwortet nicht – ist das der richtige Pfad?", path))?;
    Ok(version)
}
