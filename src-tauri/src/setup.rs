use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command as StdCmd, Stdio};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCmd;

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
    pub kind: String,
}

fn probe(prog: &str, args: &[&str]) -> Option<String> {
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
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Some(stdout);
        }

        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if !stderr.is_empty() {
            return Some(stderr);
        }
    }

    None
}

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

#[cfg(target_os = "windows")]
fn normalize_openclaw_path(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
        .unwrap_or(false)
    {
        return path;
    }

    let cmd_variant = PathBuf::from(format!("{}.cmd", path.to_string_lossy()));
    if cmd_variant.exists() {
        return cmd_variant;
    }

    let bat_variant = PathBuf::from(format!("{}.bat", path.to_string_lossy()));
    if bat_variant.exists() {
        return bat_variant;
    }

    path
}

#[cfg(not(target_os = "windows"))]
fn normalize_openclaw_path(path: PathBuf) -> PathBuf {
    path
}

fn probe_openclaw(path: &str) -> Option<String> {
    probe(path, &["--version"])
}

pub fn resolve_valid_openclaw_path(path: &str) -> Option<String> {
    let normalized = normalize_openclaw_path(PathBuf::from(path));
    let normalized_str = normalized.to_string_lossy().to_string();
    if normalized_str.is_empty() {
        return None;
    }
    if !normalized.exists() {
        return None;
    }
    if probe_openclaw(&normalized_str).is_some() {
        return Some(normalized_str);
    }
    None
}

pub fn detect_openclaw_path() -> Option<String> {
    #[cfg(target_os = "windows")]
    if let Ok(out) = StdCmd::new("cmd").args(["/c", "where", "openclaw"]).output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let selected = stdout
                .lines()
                .find(|line| line.to_lowercase().ends_with(".cmd"))
                .or_else(|| stdout.lines().next());

            if let Some(line) = selected {
                if let Some(validated) = resolve_valid_openclaw_path(line.trim()) {
                    return Some(validated);
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    if let Ok(out) = StdCmd::new("which").arg("openclaw").output() {
        if out.status.success() {
            let candidate = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !candidate.is_empty() && probe_openclaw(&candidate).is_some() {
                return Some(candidate);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();

        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm_dir = PathBuf::from(appdata).join("npm");
            candidates.push(npm_dir.join("openclaw.cmd"));
            candidates.push(npm_dir.join("openclaw"));
        }

        if let Ok(profile) = std::env::var("USERPROFILE") {
            let npm_dir = PathBuf::from(profile)
                .join("AppData")
                .join("Roaming")
                .join("npm");
            candidates.push(npm_dir.join("openclaw.cmd"));
            candidates.push(npm_dir.join("openclaw"));
        }

        for candidate in candidates {
            if let Some(validated) = resolve_valid_openclaw_path(&candidate.to_string_lossy()) {
                return Some(validated);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        for candidate in ["/usr/local/bin/openclaw", "/usr/bin/openclaw"] {
            let candidate_str = candidate.to_string();
            if probe_openclaw(&candidate_str).is_some() {
                return Some(candidate_str);
            }
        }
    }

    if probe_openclaw("openclaw").is_some() {
        return Some("openclaw".to_string());
    }

    None
}

#[tauri::command]
pub fn check_environment() -> EnvironmentStatus {
    let node_version = probe("node", &["--version"]);
    let openclaw_path = detect_openclaw_path();
    let openclaw_version = openclaw_path
        .as_deref()
        .and_then(probe_openclaw)
        .or_else(|| probe_openclaw("openclaw"));

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
        let _ = app.emit(
            "install_progress",
            InstallProgressEvent {
                line: line.to_string(),
                kind: kind.to_string(),
            },
        );
    };

    emit("npm install -g openclaw", "command");

    let mut child = npm_cmd()
        .args(["install", "-g", "openclaw"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("npm konnte nicht gestartet werden: {}", e))?;

    if let Some(stdout) = child.stdout.take() {
        let app_handle = app.clone();
        let mut lines = BufReader::new(stdout).lines();
        tauri::async_runtime::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit(
                    "install_progress",
                    InstallProgressEvent {
                        line,
                        kind: "stdout".into(),
                    },
                );
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let app_handle = app.clone();
        let mut lines = BufReader::new(stderr).lines();
        tauri::async_runtime::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_handle.emit(
                    "install_progress",
                    InstallProgressEvent {
                        line,
                        kind: "stderr".into(),
                    },
                );
            }
        });
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        emit(
            &format!("Installation fehlgeschlagen (exit code {})", code),
            "error",
        );
        return Err(format!(
            "npm install -g openclaw ist fehlgeschlagen (Exit-Code {}). Pruefe Node/npm-Rechte oder installiere OpenClaw manuell und trage danach den Pfad ein.",
            code
        ));
    }

    let path = detect_openclaw_path().ok_or_else(|| {
        "OpenClaw wurde installiert, aber danach nicht automatisch gefunden. Nutze 'Pfad angeben' und waehle z.B. C:\\Users\\<Name>\\AppData\\Roaming\\npm\\openclaw.cmd".to_string()
    })?;

    emit(&format!("openclaw installiert: {}", path), "success");
    Ok(path)
}

#[tauri::command]
pub fn validate_openclaw_path(path: String) -> Result<String, String> {
    let normalized = normalize_openclaw_path(PathBuf::from(path.trim()));
    let normalized_str = normalized.to_string_lossy().to_string();
    let version = probe_openclaw(&normalized_str).ok_or_else(|| {
        format!(
            "'{}' antwortet nicht als OpenClaw-Binary. Auf Windows ist oft die .cmd-Datei korrekt, z.B. C:\\Users\\<Name>\\AppData\\Roaming\\npm\\openclaw.cmd",
            normalized_str
        )
    })?;
    Ok(version)
}
