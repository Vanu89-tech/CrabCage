// CrabCage – v2 sandbox build
mod proxy;
mod launcher;
mod sandbox;
mod setup;

use proxy::{ProxyHandle, ProxyEvent, start_proxy};
use launcher::find_openclaw;
use sandbox::{AllowedPathRule, SandboxHandle, launch_sandboxed};
use setup::resolve_valid_openclaw_path;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{Mutex, RwLock, mpsc};

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AllowedApp {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "addedAt")]
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AllowedPath {
    pub id: String,
    pub path: String,
    pub permissions: Vec<String>,
    #[serde(rename = "addedAt")]
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AllowedDomain {
    pub id: String,
    pub domain: String,
    #[serde(rename = "addedAt")]
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrabCageConfig {
    #[serde(rename = "allowedApps", default)]
    pub allowed_apps: Vec<AllowedApp>,
    #[serde(rename = "allowedPaths", default)]
    pub allowed_paths: Vec<AllowedPath>,
    #[serde(rename = "allowedDomains", default)]
    pub allowed_domains: Vec<AllowedDomain>,
    #[serde(rename = "onboardingComplete", default)]
    pub onboarding_complete: bool,
    #[serde(rename = "openclawPath", default)]
    pub openclaw_path: Option<String>,
}

impl Default for CrabCageConfig {
    fn default() -> Self {
        CrabCageConfig {
            allowed_apps: vec![],
            allowed_paths: vec![],
            allowed_domains: vec![],
            onboarding_complete: false,
            openclaw_path: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SessionStatus {
    pub running: bool,
    pub pid: Option<u32>,
    #[serde(rename = "networkProtectionActive")]
    pub network_protection_active: bool,
    #[serde(rename = "openclawPath")]
    pub openclaw_path: Option<String>,
    #[serde(rename = "processProtectionActive")]
    pub process_protection_active: bool,
    #[serde(rename = "filesystemProtectionActive")]
    pub filesystem_protection_active: bool,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub proxy: Mutex<Option<ProxyHandle>>,
    pub session: Mutex<Option<SandboxHandle>>,
    /// Shared with the proxy task – updated when config changes
    pub allowed_domains: Arc<RwLock<Vec<String>>>,
}

// ── File paths ────────────────────────────────────────────────────────────────

fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("CrabCage")
}

fn config_path() -> PathBuf { data_dir().join("config.json") }
fn events_path() -> PathBuf { data_dir().join("events.json") }
fn debug_log_path() -> PathBuf { data_dir().join("session-debug.log") }

fn ensure_data_dir() -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())
}

fn debug_log(message: impl AsRef<str>) {
    let _ = ensure_data_dir();
    let line = format!("{}\n", message.as_ref());
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(debug_log_path())
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
}

fn detect_node_executable() -> Option<String> {
    if let Ok(out) = std::process::Command::new("cmd").args(["/c", "where", "node"]).output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(path) = stdout.lines().map(str::trim).find(|line| !line.is_empty()) {
                return Some(path.to_string());
            }
        }
    }

    let common_paths = [
        r"C:\Program Files\nodejs\node.exe",
        r"C:\Program Files (x86)\nodejs\node.exe",
    ];

    for path in common_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

struct LaunchCommand {
    executable: String,
    args: Vec<String>,
}

fn resolve_openclaw_launch_command(path: &str) -> LaunchCommand {
    let path_lc = path.to_lowercase();

    let needs_shell = path_lc.ends_with(".cmd")
        || path_lc.ends_with(".bat")
        || (!path_lc.ends_with(".exe") && {
            let cmd_sibling = format!("{}.cmd", path);
            std::path::Path::new(&cmd_sibling).exists()
        });

    if !needs_shell {
        return LaunchCommand {
            executable: path.to_string(),
            args: vec![],
        };
    }

    let target = if !path_lc.ends_with(".cmd") && !path_lc.ends_with(".bat") {
        format!("{}.cmd", path)
    } else {
        path.to_string()
    };

    let shim_path = std::path::Path::new(&target);
    if let Some(shim_dir) = shim_path.parent() {
        let script_path = shim_dir.join("node_modules").join("openclaw").join("openclaw.mjs");
        if script_path.exists() {
            let node_in_shim_dir = shim_dir.join("node.exe");
            let node_cmd = if node_in_shim_dir.exists() {
                node_in_shim_dir.to_string_lossy().to_string()
            } else if let Some(node_path) = detect_node_executable() {
                node_path
            } else {
                "node".to_string()
            };

            return LaunchCommand {
                executable: node_cmd,
                args: vec![script_path.to_string_lossy().to_string()],
            };
        }
    }

    let sysroot = std::env::var("SystemRoot")
        .unwrap_or_else(|_| r"C:\Windows".to_string());
    LaunchCommand {
        executable: format!("{}\\System32\\cmd.exe", sysroot),
        args: vec!["/c".to_string(), target],
    }
}

fn session_launch_args() -> Vec<String> {
    vec!["gateway".to_string(), "run".to_string()]
}

fn resolve_openclaw_shell_target(path: &str) -> String {
    let path_lc = path.to_lowercase();
    if path_lc.ends_with(".cmd") || path_lc.ends_with(".bat") {
        return path.to_string();
    }

    let cmd_sibling = format!("{}.cmd", path);
    if std::path::Path::new(&cmd_sibling).exists() {
        return cmd_sibling;
    }

    path.to_string()
}

fn quote_for_cmd(arg: &str) -> String {
    format!("\"{}\"", arg.replace('"', "\"\""))
}

// ── Config commands ───────────────────────────────────────────────────────────

#[tauri::command]
fn load_config() -> Result<CrabCageConfig, String> {
    let path = config_path();
    if !path.exists() {
        return Ok(CrabCageConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(
    config: CrabCageConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    ensure_data_dir()?;
    // Keep the shared domain list in sync with the saved config
    let domains: Vec<String> = config.allowed_domains.iter().map(|d| d.domain.clone()).collect();
    {
        let mut wl = state.allowed_domains.write().await;
        *wl = domains;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), content).map_err(|e| e.to_string())
}

// ── Audit commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn load_audit_log() -> Result<Vec<AuditEvent>, String> {
    let path = events_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_audit_event(event: AuditEvent) -> Result<(), String> {
    ensure_data_dir()?;
    let mut events: Vec<AuditEvent> = {
        let path = events_path();
        if path.exists() {
            let c = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&c).unwrap_or_default()
        } else {
            vec![]
        }
    };
    events.insert(0, event);
    events.truncate(500);
    let content = serde_json::to_string_pretty(&events).map_err(|e| e.to_string())?;
    fs::write(events_path(), content).map_err(|e| e.to_string())
}

// ── Session commands ──────────────────────────────────────────────────────────

#[tauri::command]
async fn get_session_status(state: tauri::State<'_, AppState>) -> Result<SessionStatus, String> {
    let session_guard = state.session.lock().await;
    let proxy_guard = state.proxy.lock().await;
    let config = load_config().unwrap_or_default();

    let (running, pid, process_protection_active, filesystem_protection_active) =
        match session_guard.as_ref() {
            Some(s) => (
                s.is_running(),
                Some(s.pid),
                s.sandbox_active,
                s.filesystem_hardening_active,
            ),
            None => (false, None, false, false),
    };

    let openclaw_path = config
        .openclaw_path
        .as_deref()
        .and_then(resolve_valid_openclaw_path)
        .or_else(find_openclaw);

    Ok(SessionStatus {
        running,
        pid,
        network_protection_active: proxy_guard.is_some(),
        openclaw_path,
        process_protection_active,
        filesystem_protection_active,
    })
}

#[tauri::command]
async fn start_session(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SessionStatus, String> {
    debug_log("start_session: begin");
    // Load current config
    let config = load_config()?;
    debug_log(format!(
        "start_session: config loaded apps={} paths={} domains={}",
        config.allowed_apps.len(),
        config.allowed_paths.len(),
        config.allowed_domains.len()
    ));

    // Sync domain whitelist
    {
        let domains: Vec<String> = config.allowed_domains.iter().map(|d| d.domain.clone()).collect();
        let mut wl = state.allowed_domains.write().await;
        *wl = domains;
    }

    // Start proxy if not already running
    {
        let mut proxy_guard = state.proxy.lock().await;
        if proxy_guard.is_none() {
            debug_log("start_session: starting proxy");
            let (tx, mut rx) = mpsc::channel::<ProxyEvent>(128);

            // Forward proxy events to the frontend via Tauri events
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = handle.emit("proxy_event", &event);
                }
            });

            let proxy = start_proxy(state.allowed_domains.clone(), tx).await?;
            *proxy_guard = Some(proxy);
            debug_log("start_session: proxy started");
        }
    }

    // Resolve OpenClaw path
    let openclaw_path = config
        .openclaw_path
        .as_deref()
        .and_then(resolve_valid_openclaw_path)
        .or_else(find_openclaw);
    debug_log(format!("start_session: openclaw_path={:?}", openclaw_path));

    // Build allowed executables from config
    let allowed_executables: Vec<String> = config.allowed_apps
        .iter()
        .map(|a| a.path.clone())
        .collect();
    let allowed_paths: Vec<AllowedPathRule> = config
        .allowed_paths
        .iter()
        .map(|path| AllowedPathRule {
            path: path.path.clone(),
            writable: path.permissions.iter().any(|permission| permission == "write"),
        })
        .collect();
    debug_log(format!(
        "start_session: allowed_executables={} allowed_paths={}",
        allowed_executables.len(),
        allowed_paths.len()
    ));

    // Launch OpenClaw inside sandbox
    let (running, pid, process_protection_active, filesystem_protection_active) =
        match &openclaw_path {
        Some(path) => {
            let mut session_guard = state.session.lock().await;
            if session_guard.as_ref().map(|s| s.is_running()).unwrap_or(false) {
                let s = session_guard.as_ref().unwrap();
                (
                    true,
                    Some(s.pid),
                    s.sandbox_active,
                    s.filesystem_hardening_active,
                )
            } else {
                let proxy_url = format!("http://127.0.0.1:{}", proxy::PROXY_PORT);
                debug_log(format!("start_session: launching with proxy_url={}", proxy_url));
                let env_pairs = vec![
                    ("HTTP_PROXY".into(),  proxy_url.clone()),
                    ("HTTPS_PROXY".into(), proxy_url.clone()),
                    ("http_proxy".into(),  proxy_url.clone()),
                    ("https_proxy".into(), proxy_url.clone()),
                    ("NO_PROXY".into(), "localhost,127.0.0.1,::1".into()),
                    ("CRABCAGE_ACTIVE".into(), "1".into()),
                    ("CRABCAGE_PROXY_PORT".into(), proxy::PROXY_PORT.to_string()),
                ];
                let mut launch_cmd = resolve_openclaw_launch_command(path);
                launch_cmd.args.extend(session_launch_args());
                debug_log(format!(
                    "start_session: launch executable={} args={:?}",
                    launch_cmd.executable,
                    launch_cmd.args
                ));
                match launch_sandboxed(
                    &launch_cmd.executable,
                    launch_cmd.args,
                    env_pairs,
                    allowed_executables,
                    allowed_paths,
                ) {
                    Ok(handle) => {
                        debug_log(format!("start_session: launch ok pid={}", handle.pid));
                        let pid = handle.pid;
                        let process_active = handle.sandbox_active;
                        let filesystem_active = handle.filesystem_hardening_active;
                        *session_guard = Some(handle);
                        (true, Some(pid), process_active, filesystem_active)
                    }
                    Err(e) => {
                        debug_log(format!("start_session: launch error={}", e));
                        return Err(e);
                    }
                }
            }
        }
        None => (false, None, false, false),
    };
    debug_log(format!(
        "start_session: completed running={} pid={:?} network=true process={} filesystem={}",
        running,
        pid,
        process_protection_active,
        filesystem_protection_active
    ));

    Ok(SessionStatus {
        running,
        pid,
        network_protection_active: true,
        openclaw_path,
        process_protection_active,
        filesystem_protection_active,
    })
}

#[tauri::command]
async fn stop_session(state: tauri::State<'_, AppState>) -> Result<SessionStatus, String> {
    // Stop OpenClaw process (sandbox stop closes Job Object → kills all children)
    {
        let mut session_guard = state.session.lock().await;
        if let Some(mut handle) = session_guard.take() {
            handle.stop();
        }
    }

    // Stop proxy
    {
        let mut proxy_guard = state.proxy.lock().await;
        if let Some(handle) = proxy_guard.take() {
            handle.stop();
        }
    }

    Ok(SessionStatus {
        running: false,
        pid: None,
        network_protection_active: false,
        openclaw_path: None,
        process_protection_active: false,
        filesystem_protection_active: false,
    })
}

#[tauri::command]
fn detect_openclaw() -> Option<String> {
    find_openclaw()
}

#[tauri::command]
fn launch_openclaw_assistant(action: String) -> Result<String, String> {
    let path = load_config()
        .ok()
        .and_then(|config| config.openclaw_path)
        .as_deref()
        .and_then(resolve_valid_openclaw_path)
        .or_else(find_openclaw)
        .ok_or_else(|| "OpenClaw wurde nicht gefunden. Richte OpenClaw zuerst in CrabCage ein.".to_string())?;

    let (args, label) = match action.as_str() {
        "onboard" => (vec!["onboard".to_string()], "Onboarding"),
        "configure" => (vec!["configure".to_string()], "Konfiguration"),
        "channels_login" => (vec!["channels".to_string(), "login".to_string()], "Channel Login"),
        "dashboard" => (vec!["dashboard".to_string()], "Dashboard"),
        _ => return Err("Unbekannte Assistant-Aktion.".to_string()),
    };

    let shell_target = resolve_openclaw_shell_target(&path);
    let mut command_parts = vec![quote_for_cmd(&shell_target)];
    command_parts.extend(args.iter().map(|arg| quote_for_cmd(arg)));
    let command_line = command_parts.join(" ");

    let status = std::process::Command::new("cmd")
        .args([
            "/c",
            "start",
            "OpenClaw Assistant",
            "cmd",
            "/k",
            &command_line,
        ])
        .status()
        .map_err(|e| format!("Assistant-Terminal konnte nicht gestartet werden: {}", e))?;

    if !status.success() {
        return Err(format!("Assistant-Terminal für '{}' konnte nicht gestartet werden.", label));
    }

    Ok(format!("OpenClaw {} wurde in einem neuen Terminal gestartet.", label))
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        proxy: Mutex::new(None),
        session: Mutex::new(None),
        allowed_domains: Arc::new(RwLock::new(vec![])),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            load_audit_log,
            add_audit_event,
            get_session_status,
            start_session,
            stop_session,
            detect_openclaw,
            launch_openclaw_assistant,
            setup::check_environment,
            setup::install_openclaw,
            setup::validate_openclaw_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CrabCage");
}
