// CrabCage – v2 sandbox build
mod proxy;
mod launcher;
mod sandbox;
mod setup;

use proxy::{ProxyHandle, ProxyEvent, start_proxy};
use launcher::find_openclaw;
use sandbox::{SandboxHandle, launch_sandboxed};

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
    #[serde(rename = "proxyActive")]
    pub proxy_active: bool,
    #[serde(rename = "openclawPath")]
    pub openclaw_path: Option<String>,
    #[serde(rename = "sandboxActive")]
    pub sandbox_active: bool,
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

fn ensure_data_dir() -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())
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

    let (running, pid, sandbox_active) = match session_guard.as_ref() {
        Some(s) => (s.is_running(), Some(s.pid), s.sandbox_active),
        None => (false, None, false),
    };

    Ok(SessionStatus {
        running,
        pid,
        proxy_active: proxy_guard.is_some(),
        openclaw_path: None,
        sandbox_active,
    })
}

#[tauri::command]
async fn start_session(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SessionStatus, String> {
    // Load current config
    let config = load_config()?;

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
        }
    }

    // Resolve OpenClaw path
    let openclaw_path = config.openclaw_path
        .as_deref()
        .map(str::to_string)
        .or_else(find_openclaw);

    // Build allowed executables from config
    let allowed_executables: Vec<String> = config.allowed_apps
        .iter()
        .map(|a| a.path.clone())
        .collect();

    // Launch OpenClaw inside sandbox
    let (running, pid, sandbox_active) = match &openclaw_path {
        Some(path) => {
            let mut session_guard = state.session.lock().await;
            if session_guard.as_ref().map(|s| s.is_running()).unwrap_or(false) {
                let s = session_guard.as_ref().unwrap();
                (true, Some(s.pid), s.sandbox_active)
            } else {
                let proxy_url = format!("http://127.0.0.1:{}", proxy::PROXY_PORT);
                let env_pairs = vec![
                    ("HTTP_PROXY".into(),  proxy_url.clone()),
                    ("HTTPS_PROXY".into(), proxy_url.clone()),
                    ("http_proxy".into(),  proxy_url.clone()),
                    ("https_proxy".into(), proxy_url.clone()),
                    ("NO_PROXY".into(), "localhost,127.0.0.1,::1".into()),
                    ("CRABCAGE_ACTIVE".into(), "1".into()),
                    ("CRABCAGE_PROXY_PORT".into(), proxy::PROXY_PORT.to_string()),
                ];
                // .cmd/.bat files (and extensionless npm shims) can't be launched
                // directly by CreateProcessW – wrap with the full cmd.exe path.
                let path_lc = path.to_lowercase();
                let needs_shell = path_lc.ends_with(".cmd")
                    || path_lc.ends_with(".bat")
                    || (!path_lc.ends_with(".exe") && {
                        // Extensionless? Check if a .cmd sibling exists (npm shim)
                        let cmd_sibling = format!("{}.cmd", path);
                        std::path::Path::new(&cmd_sibling).exists()
                    });
                let launch_cmd = if needs_shell {
                    let sysroot = std::env::var("SystemRoot")
                        .unwrap_or_else(|_| r"C:\Windows".to_string());
                    // If extensionless, use the .cmd sibling directly
                    let target = if !path_lc.ends_with(".cmd") && !path_lc.ends_with(".bat") {
                        format!("{}.cmd", path)
                    } else {
                        path.clone()
                    };
                    format!("\"{}\\System32\\cmd.exe\" /c \"{}\"", sysroot, target)
                } else {
                    path.clone()
                };
                match launch_sandboxed(&launch_cmd, env_pairs, allowed_executables) {
                    Ok(handle) => {
                        let pid = handle.pid;
                        let active = handle.sandbox_active;
                        *session_guard = Some(handle);
                        (true, Some(pid), active)
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        None => (false, None, false),
    };

    Ok(SessionStatus {
        running,
        pid,
        proxy_active: true,
        openclaw_path,
        sandbox_active,
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
        proxy_active: false,
        openclaw_path: None,
        sandbox_active: false,
    })
}

#[tauri::command]
fn detect_openclaw() -> Option<String> {
    find_openclaw()
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
            setup::check_environment,
            setup::install_openclaw,
            setup::validate_openclaw_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CrabCage");
}
