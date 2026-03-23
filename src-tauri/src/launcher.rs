use crate::setup::detect_openclaw_path;

/// Tries to find the openclaw binary in common locations.
pub fn find_openclaw() -> Option<String> {
    detect_openclaw_path()
}
