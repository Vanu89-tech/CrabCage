/// CrabCage Sandbox - Windows Job Object + AppContainer implementation.
///
/// Schutzebenen:
/// - Job Object: Prozessgruppe wird beim Stoppen vollstaendig beendet
/// - Kindprozess-Monitor: nicht erlaubte Prozesse werden beendet
/// - AppContainer: Dateisystemzugriff nur auf explizit freigegebene Pfade

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub struct AllowedPathRule {
    pub path: String,
    pub writable: bool,
}

pub struct SandboxHandle {
    pub pid: u32,
    pub sandbox_active: bool,
    pub filesystem_hardening_active: bool,
    stop_flag: Arc<AtomicBool>,
    #[cfg(windows)]
    _job: JobObject,
}

impl SandboxHandle {
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        #[cfg(windows)]
        return win::pid_is_alive(self.pid);
        #[cfg(not(windows))]
        return false;
    }
}

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

#[cfg(windows)]
unsafe impl Send for JobObject {}
#[cfg(windows)]
unsafe impl Sync for JobObject {}

pub fn launch_sandboxed(
    executable_path: &str,
    args: Vec<String>,
    env_pairs: Vec<(String, String)>,
    allowed_executables: Vec<String>,
    allowed_paths: Vec<AllowedPathRule>,
) -> Result<SandboxHandle, String> {
    #[cfg(windows)]
    return win::launch(
        executable_path,
        args,
        env_pairs,
        allowed_executables,
        allowed_paths,
    );

    #[cfg(not(windows))]
    return Err("Sandbox nur auf Windows verfuegbar".to_string());
}

fn sandbox_debug_log(message: impl AsRef<str>) {
    let appdata = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(appdata).join("CrabCage");
    let _ = std::fs::create_dir_all(&root);
    let path = root.join("session-debug.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", message.as_ref());
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::{OsStr, c_void};
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};
    use std::thread;
    use std::time::Duration;
    use serde::{Deserialize, Serialize};

    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
        WAIT_OBJECT_0,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, EXPLICIT_ACCESS_W, GetNamedSecurityInfoW, SE_FILE_OBJECT,
        SET_ACCESS, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN,
        TRUSTEE_W,
    };
    use windows_sys::Win32::Security::Isolation::{
        CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
        GetAppContainerFolderPath,
    };
    use windows_sys::Win32::Security::{
        ACL, CreateWellKnownSid, DACL_SECURITY_INFORMATION, OBJECT_INHERIT_ACE,
        PSID, SECURITY_CAPABILITIES, SECURITY_MAX_SID_SIZE, SID_AND_ATTRIBUTES,
        SUB_CONTAINERS_AND_OBJECTS_INHERIT, WinCapabilityInternetClientSid,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
        DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
        InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcess,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, ResumeThread, STARTUPINFOEXW,
        TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
    };

    const APP_CONTAINER_NAME: &str = "CrabCage.OpenClaw";

    struct AttributeList {
        _buffer: Vec<u8>,
        ptr: LPPROC_THREAD_ATTRIBUTE_LIST,
    }

    impl AttributeList {
        fn new(attribute_count: u32) -> Result<Self, String> {
            unsafe {
                let mut size = 0usize;
                InitializeProcThreadAttributeList(null_mut(), attribute_count, 0, &mut size);
                let mut buffer = vec![0u8; size];
                let ptr = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
                if InitializeProcThreadAttributeList(ptr, attribute_count, 0, &mut size) == 0 {
                    return Err(format!(
                        "ProcThreadAttributeList konnte nicht initialisiert werden: {}",
                        std::io::Error::last_os_error()
                    ));
                }
                Ok(Self { _buffer: buffer, ptr })
            }
        }
    }

    impl Drop for AttributeList {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe { DeleteProcThreadAttributeList(self.ptr) };
            }
        }
    }

    enum SidOwnership {
        UserEnv,
        Boxed(usize),
    }

    struct OwnedSid {
        sid: PSID,
        ownership: SidOwnership,
    }

    impl OwnedSid {
        fn from_userenv(sid: PSID) -> Self {
            Self {
                sid,
                ownership: SidOwnership::UserEnv,
            }
        }

        fn from_bytes(bytes: Vec<u8>) -> Self {
            let boxed = bytes.into_boxed_slice();
            let len = boxed.len();
            let raw = Box::into_raw(boxed) as *mut u8;
            Self {
                sid: raw as PSID,
                ownership: SidOwnership::Boxed(len),
            }
        }

        fn as_ptr(&self) -> PSID {
            self.sid
        }

        fn to_string(&self) -> Result<String, String> {
            unsafe {
                let mut ptr = null_mut();
                if ConvertSidToStringSidW(self.sid, &mut ptr) == 0 {
                    return Err(format!(
                        "SID konnte nicht in String umgewandelt werden: {}",
                        std::io::Error::last_os_error()
                    ));
                }
                let result = pwstr_to_string(ptr);
                LocalFree(ptr as HLOCAL);
                Ok(result)
            }
        }
    }

    impl Drop for OwnedSid {
        fn drop(&mut self) {
            if self.sid.is_null() {
                return;
            }
            unsafe {
                match self.ownership {
                    SidOwnership::UserEnv => {
                        windows_sys::Win32::Security::FreeSid(self.sid);
                    }
                    SidOwnership::Boxed(len) => {
                        let slice = std::ptr::slice_from_raw_parts_mut(self.sid as *mut u8, len);
                        drop(Box::from_raw(slice));
                    }
                }
            }
        }
    }

    struct AppContainerProfile {
        sid: OwnedSid,
        capabilities: Vec<OwnedSid>,
    }

    struct AppContainerDirs {
        home: PathBuf,
        appdata: PathBuf,
        localappdata: PathBuf,
        temp: PathBuf,
        runtime: PathBuf,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    struct AclCacheEntry {
        path: String,
        writable: bool,
        mtime: Option<u64>,
    }

    #[derive(Debug, Serialize, Deserialize, Default)]
    struct AclCache {
        entries: Vec<AclCacheEntry>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct RuntimeManifest {
        executable_source: String,
        executable_mtime: Option<u64>,
    }

    pub fn pid_is_alive(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return false;
            }
            let result = WaitForSingleObject(h, 0);
            CloseHandle(h);
            result != WAIT_OBJECT_0
        }
    }

    fn data_dir() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("CrabCage")
    }

    fn pwstr_to_string(ptr: windows_sys::core::PWSTR) -> String {
        unsafe {
            if ptr.is_null() {
                return String::new();
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
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

    fn build_env_block(overrides: &[(String, String)]) -> Vec<u16> {
        let mut normalized: HashMap<String, (String, String)> = HashMap::new();

        for (key, value) in std::env::vars() {
            if key.contains('=') {
                continue;
            }
            normalized.insert(key.to_ascii_lowercase(), (key, value));
        }

        for (key, value) in overrides {
            if key.contains('=') {
                continue;
            }
            normalized.insert(key.to_ascii_lowercase(), (key.clone(), value.clone()));
        }

        let mut entries: Vec<(String, String)> = normalized.into_values().collect();
        entries.sort_by(|(left, _), (right, _)| {
            left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
        });

        let mut block: Vec<u16> = Vec::new();
        for (key, value) in entries {
            let entry = format!("{}={}", key, value);
            block.extend(OsStr::new(&entry).encode_wide());
            block.push(0);
        }
        block.push(0);
        block
    }

    fn to_wide_null(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn quote_arg(arg: &str) -> String {
        if arg.is_empty() {
            return "\"\"".to_string();
        }
        if !arg.contains(' ') && !arg.contains('\t') && !arg.contains('"') {
            return arg.to_string();
        }
        format!("\"{}\"", arg.replace('"', "\\\""))
    }

    fn build_command_line(executable_path: &str, args: &[String]) -> String {
        let mut parts = Vec::with_capacity(args.len() + 1);
        parts.push(quote_arg(executable_path));
        parts.extend(args.iter().map(|arg| quote_arg(arg)));
        parts.join(" ")
    }

    fn ensure_dir(path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| format!("Verzeichnis {:?} konnte nicht erstellt werden: {}", path, e))
    }

    fn create_capability_sid(kind: windows_sys::Win32::Security::WELL_KNOWN_SID_TYPE) -> Result<OwnedSid, String> {
        unsafe {
            let mut size = SECURITY_MAX_SID_SIZE;
            let mut bytes = vec![0u8; size as usize];
            if CreateWellKnownSid(kind, null_mut(), bytes.as_mut_ptr() as PSID, &mut size) == 0 {
                return Err(format!(
                    "Capability-SID konnte nicht erstellt werden: {}",
                    std::io::Error::last_os_error()
                ));
            }
            bytes.truncate(size as usize);
            Ok(OwnedSid::from_bytes(bytes))
        }
    }

    fn ensure_appcontainer_profile() -> Result<AppContainerProfile, String> {
        unsafe {
            let name_w = to_wide_null(APP_CONTAINER_NAME);
            let mut sid = null_mut();
            let derive_hr = DeriveAppContainerSidFromAppContainerName(name_w.as_ptr(), &mut sid);

            if derive_hr < 0 {
                let mut created_sid = null_mut();
                let create_hr = CreateAppContainerProfile(
                    name_w.as_ptr(),
                    name_w.as_ptr(),
                    name_w.as_ptr(),
                    null(),
                    0,
                    &mut created_sid,
                );
                if create_hr < 0 {
                    return Err(format!(
                        "AppContainer-Profil konnte nicht erstellt werden (HRESULT 0x{:08X})",
                        create_hr as u32
                    ));
                }

                if created_sid.is_null() {
                    let derive_retry =
                        DeriveAppContainerSidFromAppContainerName(name_w.as_ptr(), &mut sid);
                    if derive_retry < 0 {
                        return Err(format!(
                            "AppContainer-SID konnte nicht abgeleitet werden (HRESULT 0x{:08X})",
                            derive_retry as u32
                        ));
                    }
                } else {
                    sid = created_sid;
                }
            }

            let capabilities = vec![create_capability_sid(WinCapabilityInternetClientSid)?];

            Ok(AppContainerProfile {
                sid: OwnedSid::from_userenv(sid),
                capabilities,
            })
        }
    }

    fn resolve_appcontainer_root(profile: &AppContainerProfile) -> Result<PathBuf, String> {
        let sid_string = profile.sid.to_string()?;
        let sid_w = to_wide_null(&sid_string);
        unsafe {
            let mut path_ptr = null_mut();
            let hr = GetAppContainerFolderPath(sid_w.as_ptr(), &mut path_ptr);
            if hr >= 0 && !path_ptr.is_null() {
                let path = PathBuf::from(pwstr_to_string(path_ptr));
                LocalFree(path_ptr as HLOCAL);
                sandbox_debug_log(format!(
                    "sandbox: using native appcontainer root sid={} path={}",
                    sid_string,
                    path.to_string_lossy()
                ));
                return Ok(path);
            }
            sandbox_debug_log(format!(
                "sandbox: GetAppContainerFolderPath failed sid={} hr=0x{:08X}",
                sid_string,
                hr as u32
            ));
        }

        let local_appdata = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_dir());
        let fallback = local_appdata
            .join("Packages")
            .join(APP_CONTAINER_NAME)
            .join("AC");
        sandbox_debug_log(format!(
            "sandbox: using fallback appcontainer root path={}",
            fallback.to_string_lossy()
        ));
        Ok(fallback)
    }

    fn ensure_appcontainer_dirs(profile: &AppContainerProfile) -> Result<AppContainerDirs, String> {
        let root = resolve_appcontainer_root(profile)?;
        let dirs = AppContainerDirs {
            home: root.join("home"),
            appdata: root.join("appdata"),
            localappdata: root.join("localappdata"),
            temp: root.join("temp"),
            runtime: root.join("runtime"),
        };

        ensure_dir(&root)?;
        ensure_dir(&dirs.home)?;
        ensure_dir(&dirs.appdata)?;
        ensure_dir(&dirs.localappdata)?;
        ensure_dir(&dirs.temp)?;
        ensure_dir(&dirs.runtime)?;

        Ok(dirs)
    }

    fn modified_unix_secs(path: &Path) -> Option<u64> {
        fs::metadata(path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
    }

    fn acl_cache_path(root: &Path) -> PathBuf {
        root.join(".acl-cache.json")
    }

    fn runtime_manifest_path(root: &Path) -> PathBuf {
        root.join(".runtime-manifest.json")
    }

    fn build_runtime_manifest(executable_path: &str) -> RuntimeManifest {
        RuntimeManifest {
            executable_source: executable_path.to_string(),
            executable_mtime: modified_unix_secs(Path::new(executable_path)),
        }
    }

    fn load_runtime_manifest(root: &Path) -> Option<RuntimeManifest> {
        let path = runtime_manifest_path(root);
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_runtime_manifest(root: &Path, manifest: &RuntimeManifest) -> Result<(), String> {
        let path = runtime_manifest_path(root);
        let content = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Runtime-Manifest konnte nicht serialisiert werden: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("Runtime-Manifest {:?} konnte nicht geschrieben werden: {}", path, e))
    }

    fn load_acl_cache(root: &Path) -> AclCache {
        let path = acl_cache_path(root);
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn save_acl_cache(root: &Path, cache: &AclCache) -> Result<(), String> {
        let path = acl_cache_path(root);
        let content = serde_json::to_string_pretty(cache)
            .map_err(|e| format!("ACL-Cache konnte nicht serialisiert werden: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("ACL-Cache {:?} konnte nicht geschrieben werden: {}", path, e))
    }

    fn should_apply_acl(cache: &AclCache, path: &Path, writable: bool) -> bool {
        let entry = AclCacheEntry {
            path: path.to_string_lossy().to_string(),
            writable,
            mtime: modified_unix_secs(path),
        };
        !cache.entries.iter().any(|existing| existing == &entry)
    }

    fn remember_acl(cache: &mut AclCache, path: &Path, writable: bool) {
        let entry = AclCacheEntry {
            path: path.to_string_lossy().to_string(),
            writable,
            mtime: modified_unix_secs(path),
        };
        cache.entries.retain(|existing| existing.path != entry.path);
        cache.entries.push(entry);
    }

    fn append_icacls_rule(path: &Path, sid: &str, writable: bool) -> Result<(), String> {
        append_icacls_rule_with_mode(path, sid, writable, true)
    }

    fn append_icacls_rule_with_mode(
        path: &Path,
        sid: &str,
        writable: bool,
        recursive: bool,
    ) -> Result<(), String> {
        let metadata = fs::metadata(path)
            .map_err(|e| format!("ACL-Ziel {:?} konnte nicht gelesen werden: {}", path, e))?;

        let mut cmd = std::process::Command::new("icacls");
        cmd.arg(path);

        let permission = if metadata.is_dir() {
            if writable {
                format!("*{}:(OI)(CI)(M)", sid)
            } else {
                format!("*{}:(OI)(CI)(RX)", sid)
            }
        } else if writable {
            format!("*{}:(M)", sid)
        } else {
            format!("*{}:(RX)", sid)
        };

        cmd.args(["/grant", &permission, "/C", "/L", "/Q"]);
        if metadata.is_dir() && recursive {
            cmd.arg("/T");
        }

        let output = cmd
            .output()
            .map_err(|e| format!("icacls konnte nicht gestartet werden: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "ACL-Freigabe fuer {:?} fehlgeschlagen: {}{}",
                path,
                stdout.trim(),
                if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!(" {}", stderr.trim())
                }
            ));
        }

        Ok(())
    }

    fn grant_acl_with_dacl(path: &Path, sid: PSID, writable: bool) -> Result<(), String> {
        unsafe {
            let path_w = to_wide_null(&path.to_string_lossy());
            let mut dacl: *mut ACL = null_mut();
            let mut security_descriptor = null_mut();
            let get_result = GetNamedSecurityInfoW(
                path_w.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut security_descriptor,
            );
            if get_result != 0 {
                return Err(format!(
                    "Bestehende ACL fuer {:?} konnte nicht gelesen werden: {}",
                    path, get_result
                ));
            }

            let permissions = if writable {
                (GENERIC_READ | GENERIC_WRITE) as u32
            } else {
                GENERIC_READ as u32
            };

            let inheritance = if path.is_dir() {
                SUB_CONTAINERS_AND_OBJECTS_INHERIT
            } else {
                OBJECT_INHERIT_ACE
            };

            let mut access = EXPLICIT_ACCESS_W {
                grfAccessPermissions: permissions,
                grfAccessMode: SET_ACCESS,
                grfInheritance: inheritance,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: null_mut(),
                    MultipleTrusteeOperation: 0,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: sid as windows_sys::core::PWSTR,
                },
            };

            let mut new_acl = null_mut();
            let set_entries_result = SetEntriesInAclW(1, &mut access, dacl, &mut new_acl);
            if set_entries_result != 0 {
                if !security_descriptor.is_null() {
                    LocalFree(security_descriptor as HLOCAL);
                }
                return Err(format!(
                    "Neue ACL fuer {:?} konnte nicht erstellt werden: {}",
                    path, set_entries_result
                ));
            }

            let set_result = SetNamedSecurityInfoW(
                path_w.as_ptr() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                new_acl,
                null_mut(),
            );

            if !new_acl.is_null() {
                LocalFree(new_acl as HLOCAL);
            }
            if !security_descriptor.is_null() {
                LocalFree(security_descriptor as HLOCAL);
            }

            if set_result != 0 {
                return Err(format!(
                    "ACL fuer {:?} konnte nicht gesetzt werden: {}",
                    path, set_result
                ));
            }

            Ok(())
        }
    }

    fn grant_parent_chain_access(path: &Path, sid: PSID, sid_string: &str) -> Result<(), String> {
        let user_profile = std::env::var("USERPROFILE").ok().map(PathBuf::from);
        let appdata = std::env::var("APPDATA").ok().map(PathBuf::from);
        let local_appdata = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from);

        let mut current = if path.is_dir() {
            path.parent()
        } else {
            path.parent()
        };

        let mut ancestors = Vec::new();
        while let Some(dir) = current {
            ancestors.push(dir.to_path_buf());
            current = dir.parent();
        }

        ancestors.reverse();

        for ancestor in ancestors {
            if !ancestor.exists() {
                continue;
            }

            let in_safe_scope = user_profile
                .as_ref()
                .map(|base| ancestor.starts_with(base))
                .unwrap_or(false)
                || appdata
                    .as_ref()
                    .map(|base| ancestor.starts_with(base))
                    .unwrap_or(false)
                || local_appdata
                    .as_ref()
                    .map(|base| ancestor.starts_with(base))
                    .unwrap_or(false);

            if !in_safe_scope {
                continue;
            }

            if user_profile
                .as_ref()
                .map(|base| ancestor == *base)
                .unwrap_or(false)
            {
                continue;
            }

            // Traverse/read on parent directories only. No recursive broadening here.
            grant_acl_with_dacl(&ancestor, sid, false)?;
            append_icacls_rule_with_mode(&ancestor, sid_string, false, false)?;
        }

        Ok(())
    }

    fn collect_runtime_paths(executable_path: &str, args: &[String]) -> Vec<(PathBuf, bool)> {
        let mut paths = Vec::new();

        let executable = PathBuf::from(executable_path);
        if executable.exists() {
            paths.push((executable.clone(), false));
            if let Some(parent) = executable.parent() {
                paths.push((parent.to_path_buf(), false));
            }
        }

        for arg in args {
            let candidate = PathBuf::from(arg);
            if candidate.exists() {
                let writable = false;
                paths.push((candidate.clone(), writable));
                if candidate
                    .file_name()
                    .map(|name| name.to_string_lossy().eq_ignore_ascii_case("openclaw.mjs"))
                    .unwrap_or(false)
                {
                    if let Some(parent) = candidate.parent() {
                        paths.push((parent.to_path_buf(), false));
                    }
                }
            }
        }

        paths.sort_by(|a, b| a.0.cmp(&b.0));
        paths.dedup_by(|a, b| a.0 == b.0);
        paths
    }

    fn is_implicitly_readable_system_path(path: &Path) -> bool {
        let system_roots = [
            std::env::var("ProgramFiles").ok().map(PathBuf::from),
            std::env::var("ProgramFiles(x86)").ok().map(PathBuf::from),
            std::env::var("SystemRoot").ok().map(PathBuf::from),
            std::env::var("WINDIR").ok().map(PathBuf::from),
        ];

        system_roots
            .into_iter()
            .flatten()
            .any(|root| path.starts_with(root))
    }

    fn prepare_filesystem_access(
        executable_path: &str,
        args: &[String],
        allowed_paths: &[AllowedPathRule],
        profile: &AppContainerProfile,
    ) -> Result<(AppContainerDirs, String), String> {
        let dirs = ensure_appcontainer_dirs(profile)?;
        sandbox_debug_log(format!("sandbox: dirs ready runtime={}", dirs.runtime.to_string_lossy()));
        let staged_executable = executable_path.to_string();
        sandbox_debug_log(format!(
            "sandbox: using original runtime executable={}",
            staged_executable
        ));
        let sid_string = profile.sid.to_string()?;
        sandbox_debug_log(format!("sandbox: sid string ready sid={}", sid_string));
        let mut acl_cache = load_acl_cache(&dirs.runtime);
        sandbox_debug_log(format!("sandbox: acl cache entries={}", acl_cache.entries.len()));

        for path in [
            dirs.home.as_path(),
            dirs.appdata.as_path(),
            dirs.localappdata.as_path(),
            dirs.temp.as_path(),
            dirs.runtime.as_path(),
        ] {
            if should_apply_acl(&acl_cache, path, true) {
                sandbox_debug_log(format!("sandbox: applying writable acl to managed path={}", path.to_string_lossy()));
                grant_parent_chain_access(path, profile.sid.as_ptr(), &sid_string)?;
                grant_acl_with_dacl(path, profile.sid.as_ptr(), true)?;
                append_icacls_rule(path, &sid_string, true)?;
                remember_acl(&mut acl_cache, path, true);
            } else {
                sandbox_debug_log(format!("sandbox: managed path cached={}", path.to_string_lossy()));
            }
        }

        for (path, writable) in collect_runtime_paths(&staged_executable, args) {
            if !writable && is_implicitly_readable_system_path(&path) {
                sandbox_debug_log(format!(
                    "sandbox: skipping acl for system runtime path={}",
                    path.to_string_lossy()
                ));
                remember_acl(&mut acl_cache, &path, writable);
                continue;
            }
            if should_apply_acl(&acl_cache, &path, writable) {
                sandbox_debug_log(format!(
                    "sandbox: applying runtime acl path={} writable={}",
                    path.to_string_lossy(),
                    writable
                ));
                grant_parent_chain_access(&path, profile.sid.as_ptr(), &sid_string)?;
                grant_acl_with_dacl(&path, profile.sid.as_ptr(), writable)?;
                append_icacls_rule(&path, &sid_string, writable)?;
                remember_acl(&mut acl_cache, &path, writable);
            } else {
                sandbox_debug_log(format!(
                    "sandbox: runtime path cached={} writable={}",
                    path.to_string_lossy(),
                    writable
                ));
            }
        }

        for rule in allowed_paths {
            let path = PathBuf::from(&rule.path);
            if !path.exists() {
                sandbox_debug_log(format!("sandbox: allowed path missing={}", path.to_string_lossy()));
                continue;
            }
            if should_apply_acl(&acl_cache, &path, rule.writable) {
                sandbox_debug_log(format!(
                    "sandbox: applying allowed path acl path={} writable={}",
                    path.to_string_lossy(),
                    rule.writable
                ));
                grant_parent_chain_access(&path, profile.sid.as_ptr(), &sid_string)?;
                grant_acl_with_dacl(&path, profile.sid.as_ptr(), rule.writable)?;
                append_icacls_rule(&path, &sid_string, rule.writable)?;
                remember_acl(&mut acl_cache, &path, rule.writable);
            } else {
                sandbox_debug_log(format!(
                    "sandbox: allowed path cached={} writable={}",
                    path.to_string_lossy(),
                    rule.writable
                ));
            }
        }

        save_acl_cache(&dirs.runtime, &acl_cache)?;
        sandbox_debug_log("sandbox: acl cache saved");

        Ok((dirs, staged_executable))
    }

    fn build_security_capabilities(
        profile: &AppContainerProfile,
    ) -> (SECURITY_CAPABILITIES, Vec<SID_AND_ATTRIBUTES>) {
        let capabilities = profile
            .capabilities
            .iter()
            .map(|sid| SID_AND_ATTRIBUTES {
                Sid: sid.as_ptr(),
                Attributes: 0,
            })
            .collect::<Vec<_>>();

        let security = SECURITY_CAPABILITIES {
            AppContainerSid: profile.sid.as_ptr(),
            Capabilities: capabilities.as_ptr() as *mut SID_AND_ATTRIBUTES,
            CapabilityCount: capabilities.len() as u32,
            Reserved: 0,
        };

        (security, capabilities)
    }

    unsafe fn create_appcontainer_process(
        application_name: &str,
        args: &[String],
        env_pairs: &[(String, String)],
        current_directory: &str,
        attribute_list: LPPROC_THREAD_ATTRIBUTE_LIST,
        suspended: bool,
    ) -> Result<PROCESS_INFORMATION, std::io::Error> {
        let command_line = build_command_line(application_name, args);
        let mut command_w = to_wide_null(&command_line);
        let application_w = to_wide_null(application_name);
        let current_directory_w = to_wide_null(current_directory);
        let env_block = build_env_block(env_pairs);

        let mut si: STARTUPINFOEXW = std::mem::zeroed();
        si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attribute_list;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

        let mut creation_flags =
            CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT;
        if suspended {
            creation_flags |= CREATE_SUSPENDED;
        }

        let created = CreateProcessW(
            application_w.as_ptr(),
            command_w.as_mut_ptr(),
            null(),
            null(),
            0,
            creation_flags,
            env_block.as_ptr() as *const c_void,
            current_directory_w.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        );

        if created == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(pi)
        }
    }

    pub fn launch(
        executable_path: &str,
        args: Vec<String>,
        env_pairs: Vec<(String, String)>,
        allowed_executables: Vec<String>,
        allowed_paths: Vec<AllowedPathRule>,
    ) -> Result<SandboxHandle, String> {
        sandbox_debug_log(format!("sandbox: launch begin executable={} args={:?}", executable_path, args));
        unsafe {
            let job_raw = CreateJobObjectW(null(), null());
            if job_raw.is_null() {
                return Err("Job Object konnte nicht erstellt werden".to_string());
            }

            let mut ext: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            ext.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job_raw,
                JobObjectExtendedLimitInformation,
                &ext as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job_raw);
                return Err("Job Object konfigurieren fehlgeschlagen".to_string());
            }

            let profile = ensure_appcontainer_profile()?;
            sandbox_debug_log("sandbox: appcontainer profile ready");
            let (dirs, staged_executable) =
                prepare_filesystem_access(executable_path, &args, &allowed_paths, &profile)?;
            sandbox_debug_log(format!(
                "sandbox: filesystem prepared executable={} staged_executable={} args={:?}",
                executable_path,
                staged_executable,
                args
            ));

            let mut env_pairs = env_pairs;
            let staged_executable_parent = Path::new(&staged_executable)
                .parent()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| dirs.runtime.to_string_lossy().to_string());
            let runtime_launch_dir = dirs.runtime.join("launch").to_string_lossy().to_string();
            let existing_path = std::env::var("PATH").unwrap_or_default();
            let merged_path = if existing_path.is_empty() {
                format!("{};{}", staged_executable_parent, runtime_launch_dir)
            } else {
                format!("{};{};{}", staged_executable_parent, runtime_launch_dir, existing_path)
            };
            env_pairs.extend_from_slice(&[
                ("HOME".into(), dirs.home.to_string_lossy().to_string()),
                ("USERPROFILE".into(), dirs.home.to_string_lossy().to_string()),
                ("APPDATA".into(), dirs.appdata.to_string_lossy().to_string()),
                (
                    "LOCALAPPDATA".into(),
                    dirs.localappdata.to_string_lossy().to_string(),
                ),
                ("TEMP".into(), dirs.temp.to_string_lossy().to_string()),
                ("TMP".into(), dirs.temp.to_string_lossy().to_string()),
                ("CRABCAGE_APP_CONTAINER".into(), APP_CONTAINER_NAME.into()),
                ("CRABCAGE_FS_HARDENED".into(), "1".into()),
                ("PATH".into(), merged_path),
            ]);

            let current_directory = staged_executable_parent.clone();

            let attribute_list = AttributeList::new(1)?;
            let (mut security_capabilities, capability_attrs) =
                build_security_capabilities(&profile);

            if UpdateProcThreadAttribute(
                attribute_list.ptr,
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                &mut security_capabilities as *mut _ as *mut c_void,
                std::mem::size_of::<SECURITY_CAPABILITIES>(),
                null_mut(),
                null(),
            ) == 0
            {
                let os_error = std::io::Error::last_os_error();
                sandbox_debug_log("sandbox: UpdateProcThreadAttribute failed");
                CloseHandle(job_raw);
                return Err(format!(
                    "AppContainer-Attribute konnten nicht gesetzt werden: {}",
                    os_error
                ));
            }

            sandbox_debug_log(format!(
                "sandbox: selftest begin executable={} current_directory={}",
                staged_executable, current_directory
            ));
            let selftest_args = vec!["-v".to_string()];
            match create_appcontainer_process(
                &staged_executable,
                &selftest_args,
                &env_pairs,
                &current_directory,
                attribute_list.ptr,
                true,
            ) {
                Ok(selftest_pi) => {
                    sandbox_debug_log(format!(
                        "sandbox: selftest create ok pid={}",
                        selftest_pi.dwProcessId
                    ));
                    TerminateProcess(selftest_pi.hProcess, 0);
                    CloseHandle(selftest_pi.hProcess);
                    CloseHandle(selftest_pi.hThread);
                }
                Err(os_error) => {
                    drop(capability_attrs);
                    drop(attribute_list);
                    CloseHandle(job_raw);
                    return Err(format!(
                        "AppContainer-Selbsttest fehlgeschlagen: {} {} ({})",
                        staged_executable,
                        selftest_args.join(" "),
                        os_error
                    ));
                }
            }

            sandbox_debug_log(format!(
                "sandbox: create begin executable={} current_directory={} args={:?}",
                staged_executable, current_directory, args
            ));
            let pi = match create_appcontainer_process(
                &staged_executable,
                &args,
                &env_pairs,
                &current_directory,
                attribute_list.ptr,
                true,
            ) {
                Ok(pi) => pi,
                Err(os_error) => {
                    drop(capability_attrs);
                    drop(attribute_list);
                    sandbox_debug_log(format!(
                        "sandbox: CreateProcessW failed for {} {:?}",
                        staged_executable, args
                    ));
                    CloseHandle(job_raw);
                    return Err(format!(
                        "OpenClaw konnte nicht im AppContainer gestartet werden: {} {} ({})",
                        staged_executable,
                        args.join(" "),
                        os_error
                    ));
                }
            };

            drop(capability_attrs);
            drop(attribute_list);

            if pi.hProcess.is_null() || pi.hThread.is_null() {
                sandbox_debug_log(format!(
                    "sandbox: CreateProcessW returned invalid handles for {} {:?}",
                    staged_executable, args
                ));
                CloseHandle(job_raw);
                return Err("OpenClaw konnte im AppContainer keine gueltigen Prozess-Handles erhalten".to_string());
            }

            let assigned = AssignProcessToJobObject(job_raw, pi.hProcess);
            if assigned == 0 {
                sandbox_debug_log("sandbox: AssignProcessToJobObject failed");
                TerminateProcess(pi.hProcess, 1);
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
                CloseHandle(job_raw);
                return Err("Prozess dem Job Object zuweisen fehlgeschlagen".to_string());
            }

            ResumeThread(pi.hThread);
            CloseHandle(pi.hThread);

            let pid = pi.dwProcessId;
            sandbox_debug_log(format!("sandbox: process created pid={}", pid));
            CloseHandle(pi.hProcess);

            let mut always_allowed: Vec<String> = vec![
                "node.exe".into(),
                "node".into(),
                "conhost.exe".into(),
                "cmd.exe".into(),
                "powershell.exe".into(),
                "powershell".into(),
                "pwsh.exe".into(),
                "pwsh".into(),
                "whoami.exe".into(),
                "whoami".into(),
                "icacls.exe".into(),
                "icacls".into(),
                "taskkill.exe".into(),
                "taskkill".into(),
            ];

            if let Some(name) = Path::new(&staged_executable).file_name() {
                always_allowed.push(name.to_string_lossy().to_lowercase());
            }

            let allowed_lower: Vec<String> = allowed_executables
                .iter()
                .map(|s| {
                    Path::new(s)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_else(|| s.to_lowercase())
                })
                .chain(always_allowed.into_iter())
                .collect();

            let stop_flag = Arc::new(AtomicBool::new(false));
            let stop_clone = stop_flag.clone();

            thread::spawn(move || {
                let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
                seen.insert(pid);

                while !stop_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(500));

                    for child_pid in child_pids_recursive(pid) {
                        if seen.contains(&child_pid) {
                            continue;
                        }
                        seen.insert(child_pid);

                        if let Some(name) = get_process_name(child_pid) {
                            let ok = allowed_lower.iter().any(|allowed| allowed == &name);
                            if !ok {
                                let h = OpenProcess(PROCESS_TERMINATE, 0, child_pid);
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
                filesystem_hardening_active: true,
                stop_flag,
                _job: JobObject(job_raw),
            })
        }
    }
}
