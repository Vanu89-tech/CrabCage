/// CrabCage Sandbox – Windows Job Object implementation.
///
/// What it does:
/// - Groups OpenClaw + ALL child processes in a Job Object
/// - KILL_ON_JOB_CLOSE: entire process tree dies when session stops
/// - Background monitor: kills unauthorized child processes every 500 ms
///
/// What it does NOT do:
/// - File system path restriction (requires AppContainer – future work)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Public types (cross-platform) ────────────────────────────────────────────

pub struct SandboxHandle {
    pub pid: u32,
    pub sandbox_active: bool,
    stop_flag: Arc<AtomicBool>,
    #[cfg(windows)]
    job: JobObject,
}

impl SandboxHandle {
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        // On Windows: dropping JobObject closes the handle → KILL_ON_JOB_CLOSE fires
    }

    pub fn is_running(&self) -> bool {
        #[cfg(windows)]
        return win::pid_is_alive(self.pid);
        #[cfg(not(windows))]
        return false;
    }
}

// ── Windows-specific implementation ──────────────────────────────────────────

#[cfg(windows)]
struct JobObject(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for JobObject {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
        }
    }
}

// HANDLE is *mut c_void – safe to send across threads in our controlled usage
#[cfg(windows)]
unsafe impl Send for JobObject {}
#[cfg(windows)]
unsafe impl Sync for JobObject {}

// ── Launch ───────────────────────────────────────────────────────────────────

/// Launch `exe_path` inside a Windows Job Object sandbox.
/// `env_pairs` are additional environment variables (proxy settings etc.).
/// `allowed_executables` are filenames (e.g. "node.exe") that child processes
/// are allowed to be. Everything else gets terminated immediately.
pub fn launch_sandboxed(
    exe_path: &str,
    env_pairs: Vec<(String, String)>,
    allowed_executables: Vec<String>,
) -> Result<SandboxHandle, String> {
    #[cfg(windows)]
    return win::launch(exe_path, env_pairs, allowed_executables);

    #[cfg(not(windows))]
    return Err("Sandbox nur auf Windows verfügbar".to_string());
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::thread;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, OpenProcess, ResumeThread, TerminateProcess, WaitForSingleObject,
        CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED, PROCESS_INFORMATION,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
    };

    // ── Process helpers ───────────────────────────────────────────────────────

    pub fn pid_is_alive(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return false;
            }
            let result = WaitForSingleObject(h, 0);
            CloseHandle(h);
            result != WAIT_OBJECT_0 // WAIT_OBJECT_0 means process exited
        }
    }

    fn get_process_name(pid: u32) -> Option<String> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    if entry.th32ProcessID == pid {
                        CloseHandle(snap);
                        let name: String = entry
                            .szExeFile
                            .iter()
                            .take_while(|&&c| c != 0)
                            .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                            .collect();
                        return Some(name.to_lowercase());
                    }
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
            None
        }
    }

    /// Returns all direct child PIDs of `parent_pid`.
    fn child_pids(parent_pid: u32) -> Vec<u32> {
        let mut result = Vec::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return result;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    if entry.th32ParentProcessID == parent_pid {
                        result.push(entry.th32ProcessID);
                    }
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
        }
        result
    }

    fn child_pids_recursive(root: u32) -> Vec<u32> {
        let mut all = Vec::new();
        for pid in child_pids(root) {
            all.push(pid);
            all.extend(child_pids_recursive(pid));
        }
        all
    }

    // ── Environment block ─────────────────────────────────────────────────────

    /// Build a Windows environment block: KEY=VALUE\0KEY=VALUE\0\0
    /// Inherits current process environment, then applies overrides.
    fn build_env_block(overrides: &[(String, String)]) -> Vec<u16> {
        let mut map: std::collections::HashMap<String, String> =
            std::env::vars().collect();
        for (k, v) in overrides {
            map.insert(k.clone(), v.clone());
        }
        let mut block: Vec<u16> = Vec::new();
        for (k, v) in &map {
            let entry = format!("{}={}", k, v);
            block.extend(OsStr::new(&entry).encode_wide());
            block.push(0);
        }
        block.push(0); // double-null terminator
        block
    }

    // ── Launch ────────────────────────────────────────────────────────────────

    pub fn launch(
        exe_path: &str,
        env_pairs: Vec<(String, String)>,
        allowed_executables: Vec<String>,
    ) -> Result<SandboxHandle, String> {
        unsafe {
            // 1. Create Job Object
            let job_raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job_raw.is_null() {
                return Err("Job Object konnte nicht erstellt werden".to_string());
            }

            // 2. KILL_ON_JOB_CLOSE → process tree dies when job handle closes
            let mut ext: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            ext.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job_raw,
                JobObjectExtendedLimitInformation,
                &ext as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job_raw);
                return Err("Job Object konfigurieren fehlgeschlagen".to_string());
            }

            // 3. Build command line (Windows CreateProcessW requires mutable buffer)
            let mut cmd_wide: Vec<u16> = OsStr::new(exe_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            // 4. Build environment block
            let env_block = build_env_block(&env_pairs);

            // 5. Launch process SUSPENDED → assign to job before first instruction runs
            let mut si: STARTUPINFOW = std::mem::zeroed();
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

            let created = CreateProcessW(
                std::ptr::null(),           // lpApplicationName (use command line)
                cmd_wide.as_mut_ptr(),      // lpCommandLine (mutable)
                std::ptr::null(),           // lpProcessAttributes
                std::ptr::null(),           // lpThreadAttributes
                0,                          // bInheritHandles = false
                CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP,
                env_block.as_ptr() as *const std::ffi::c_void,
                std::ptr::null(),           // lpCurrentDirectory
                &si,
                &mut pi,
            );

            if created == 0 {
                CloseHandle(job_raw);
                return Err(format!(
                    "OpenClaw konnte nicht gestartet werden: {}",
                    exe_path
                ));
            }

            // 6. Assign to Job BEFORE resuming (no race condition possible)
            let assigned = AssignProcessToJobObject(job_raw, pi.hProcess);
            if assigned == 0 {
                TerminateProcess(pi.hProcess, 1);
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
                CloseHandle(job_raw);
                return Err("Prozess dem Job Object zuweisen fehlgeschlagen".to_string());
            }

            // 7. Resume – process starts running
            ResumeThread(pi.hThread);
            CloseHandle(pi.hThread);

            let pid = pi.dwProcessId;
            CloseHandle(pi.hProcess); // Job Object tracks it; we don't need this handle

            // 8. Build allowed list (lowercase exe names)
            let mut always_allowed: Vec<String> = vec![
                "node.exe".into(),
                "node".into(),
                "conhost.exe".into(), // Windows console host, always needed
                "cmd.exe".into(),     // needed when launching via cmd.exe /c wrapper
            ];
            // Allow the main executable itself
            if let Some(name) = std::path::Path::new(exe_path).file_name() {
                always_allowed.push(name.to_string_lossy().to_lowercase());
            }
            let allowed_lower: Vec<String> = allowed_executables
                .iter()
                .map(|s| {
                    std::path::Path::new(s)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_else(|| s.to_lowercase())
                })
                .chain(always_allowed.into_iter())
                .collect();

            // 9. Start background monitor (shell command filter)
            let stop_flag = Arc::new(AtomicBool::new(false));
            let stop_clone = stop_flag.clone();

            thread::spawn(move || {
                let mut seen: std::collections::HashSet<u32> =
                    std::collections::HashSet::new();
                seen.insert(pid);

                while !stop_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(500));

                    for child_pid in child_pids_recursive(pid) {
                        if seen.contains(&child_pid) {
                            continue;
                        }
                        seen.insert(child_pid);

                        if let Some(name) = get_process_name(child_pid) {
                            let ok = allowed_lower
                                .iter()
                                .any(|a| a == &name);
                            if !ok {
                                // Kill unauthorized process
                                let h =
                                    OpenProcess(PROCESS_TERMINATE, 0, child_pid);
                                if !h.is_null() {
                                    TerminateProcess(h, 1);
                                    CloseHandle(h);
                                }
                            }
                        }
                    }
                }
            });

            Ok(SandboxHandle {
                pid,
                sandbox_active: true,
                stop_flag,
                job: JobObject(job_raw),
            })
        }
    }
}
