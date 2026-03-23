import { invoke } from "@tauri-apps/api/core";
import type { CrabCageConfig, AuditEvent, EnvironmentStatus } from "./types";

export async function loadConfig(): Promise<CrabCageConfig> {
  return invoke<CrabCageConfig>("load_config");
}

export async function saveConfig(config: CrabCageConfig): Promise<void> {
  return invoke("save_config", { config });
}

export async function loadAuditLog(): Promise<AuditEvent[]> {
  return invoke<AuditEvent[]>("load_audit_log");
}

export async function addAuditEvent(event: AuditEvent): Promise<void> {
  return invoke("add_audit_event", { event });
}

// ── Setup ─────────────────────────────────────────────────────────────────────

export async function checkEnvironment(): Promise<EnvironmentStatus> {
  return invoke<EnvironmentStatus>("check_environment");
}

/** Streams progress via 'install_progress' events, resolves with detected path. */
export async function installOpenClaw(): Promise<string> {
  return invoke<string>("install_openclaw");
}

/** Validates a manually entered path. Returns the version string on success. */
export async function validateOpenClawPath(path: string): Promise<string> {
  return invoke<string>("validate_openclaw_path", { path });
}
