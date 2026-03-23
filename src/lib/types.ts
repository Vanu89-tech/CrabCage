export interface AllowedApp {
  id: string;
  name: string;
  path: string;
  addedAt: string;
}

export interface AllowedPath {
  id: string;
  path: string;
  permissions: ("read" | "write")[];
  addedAt: string;
}

export interface AllowedDomain {
  id: string;
  domain: string;
  addedAt: string;
}

export interface CrabCageConfig {
  allowedApps: AllowedApp[];
  allowedPaths: AllowedPath[];
  allowedDomains: AllowedDomain[];
  onboardingComplete: boolean;
  openclawPath?: string;
}

export interface EnvironmentStatus {
  nodeInstalled: boolean;
  nodeVersion: string | null;
  openclawInstalled: boolean;
  openclawVersion: string | null;
  openclawPath: string | null;
}

export interface InstallProgressEvent {
  line: string;
  type: "command" | "stdout" | "stderr" | "success" | "error";
}

export type AuditResult = "allowed" | "blocked" | "confirmed";

export interface AuditEvent {
  id: string;
  timestamp: string;
  action: string;
  resource: string;
  result: AuditResult;
  details?: string;
}
