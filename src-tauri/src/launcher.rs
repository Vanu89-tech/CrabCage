use std::process::Command;

/// Tries to find the openclaw binary in common locations.
pub fn find_openclaw() -> Option<String> {
    let candidates: &[&str] = &[
        "openclaw",
        "openclaw.cmd",
        r"C:\Users\andru\AppData\Roaming\npm\openclaw.cmd",
        r"C:\Users\andru\AppData\Roaming\npm\openclaw",
        "/usr/local/bin/openclaw",
        "/usr/bin/openclaw",
    ];

    for &path in candidates {
        if Command::new(path).arg("--version").output().is_ok() {
            return Some(path.to_string());
        }
    }
    None
}
