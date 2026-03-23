import { useEffect, useState } from "react";
import {
  AlertTriangle,
  FolderLock,
  FolderOpen,
  Info,
  Lock,
  Play,
  ShieldCheck,
  Square,
  Unlock,
  Wifi,
  WifiOff,
} from "lucide-react";
import { useConfigStore } from "../store/configStore";
import { useSessionStore } from "../store/sessionStore";
import { SessionStatusBadge } from "../components/ui/StatusBadge";
import { OpenClawSetup } from "../components/ui/OpenClawSetup";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";

function ProtectionRow({
  label,
  active,
  activeLabel,
  inactiveLabel,
  activeIcon: ActiveIcon,
  inactiveIcon: InactiveIcon,
}: {
  label: string;
  active: boolean;
  activeLabel: string;
  inactiveLabel: string;
  activeIcon: React.ElementType;
  inactiveIcon: React.ElementType;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "10px 14px",
        background: active ? "#F0FDF4" : "#F8FAFC",
        border: `1px solid ${active ? "#BBF7D0" : "#E2E8F0"}`,
        borderRadius: 8,
        fontSize: 13,
      }}
    >
      {active ? <ActiveIcon size={15} color="#16A34A" /> : <InactiveIcon size={15} color="#94A3B8" />}
      <span style={{ color: active ? "#15803D" : "#64748B", fontWeight: 500 }}>{label}</span>
      <span style={{ color: active ? "#16A34A" : "#94A3B8", marginLeft: "auto", fontSize: 12 }}>
        {active ? activeLabel : inactiveLabel}
      </span>
    </div>
  );
}

function OpenClawPathRow({ path }: { path: string | null }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "10px 14px",
        background: "#F8FAFC",
        border: "1px solid #E2E8F0",
        borderRadius: 8,
        fontSize: 13,
      }}
    >
      <Info size={15} color="#64748B" />
      <span style={{ color: "#64748B", fontWeight: 500 }}>OpenClaw</span>
      <span
        style={{
          color: path ? "#0F172A" : "#94A3B8",
          marginLeft: "auto",
          fontSize: 12,
          fontFamily: "monospace",
          maxWidth: 260,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {path ?? "Nicht gefunden"}
      </span>
    </div>
  );
}

export function SessionControl() {
  const { config, initialized, fetchConfig } = useConfigStore();
  const { status, loading, error, fetchStatus, startSession, stopSession } = useSessionStore();
  const [confirmStop, setConfirmStop] = useState(false);

  useEffect(() => {
    fetchConfig();
    fetchStatus();
  }, [fetchConfig, fetchStatus]);

  const openclawReady = !initialized || Boolean(status.openclawPath);
  const hasNoRules =
    config.allowedApps.length === 0 &&
    config.allowedPaths.length === 0 &&
    config.allowedDomains.length === 0;

  return (
    <div style={{ maxWidth: 600 }}>
      {confirmStop && (
        <ConfirmDialog
          title="Session wirklich beenden?"
          message="OpenClaw wird gestoppt und der Proxy abgeschaltet."
          confirmLabel="Session beenden"
          onConfirm={() => {
            setConfirmStop(false);
            stopSession();
          }}
          onCancel={() => setConfirmStop(false)}
        />
      )}

      <div style={{ marginBottom: 28 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: "0 0 6px" }}>Session starten</h1>
        <p style={{ fontSize: 14, color: "#64748B", margin: 0 }}>
          OpenClaw laeuft nur innerhalb deiner Sicherheitsgrenzen. CrabCage trennt dabei sauber Netzwerk,
          Prozesse und Dateisystem.
        </p>
      </div>

      {!openclawReady && (
        <div style={{ marginBottom: 20 }}>
          <OpenClawSetup onDone={() => fetchConfig()} />
        </div>
      )}

      <div
        style={{
          background: "white",
          border: "1px solid #E2E8F0",
          borderRadius: 12,
          padding: "28px 32px",
          textAlign: "center",
          marginBottom: 16,
        }}
      >
        <div style={{ marginBottom: 20 }}>
          <SessionStatusBadge active={status.running} />
        </div>

        <div
          style={{
            width: 80,
            height: 80,
            borderRadius: "50%",
            background: status.running ? "#DCFCE7" : "#F1F5F9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            margin: "0 auto 12px",
            transition: "all 0.3s ease",
          }}
        >
          <ShieldCheck size={36} color={status.running ? "#16A34A" : "#94A3B8"} />
        </div>

        {status.running && status.pid && (
          <p style={{ fontSize: 12, color: "#94A3B8", margin: "0 0 20px", fontFamily: "monospace" }}>PID {status.pid}</p>
        )}
        {!status.running && <div style={{ marginBottom: 20 }} />}

        {status.running ? (
          <button
            onClick={() => setConfirmStop(true)}
            disabled={loading}
            style={{
              padding: "14px 40px",
              background: "#DC2626",
              color: "white",
              border: "none",
              borderRadius: 10,
              fontSize: 15,
              fontWeight: 600,
              cursor: "pointer",
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              opacity: loading ? 0.7 : 1,
            }}
          >
            <Square size={16} />
            {loading ? "Wird gestoppt..." : "Session beenden"}
          </button>
        ) : (
          <button
            onClick={startSession}
            disabled={loading || hasNoRules}
            style={{
              padding: "14px 40px",
              background: hasNoRules ? "#E2E8F0" : "#4F46E5",
              color: hasNoRules ? "#94A3B8" : "white",
              border: "none",
              borderRadius: 10,
              fontSize: 15,
              fontWeight: 600,
              cursor: hasNoRules ? "not-allowed" : "pointer",
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              opacity: loading ? 0.7 : 1,
            }}
          >
            <Play size={16} />
            {loading ? "Wird gestartet..." : "OpenClaw starten"}
          </button>
        )}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
        <ProtectionRow
          label="Netzwerk geschuetzt"
          active={status.networkProtectionActive}
          activeLabel="Aktiv - Proxy auf Port 18080"
          inactiveLabel="Inaktiv"
          activeIcon={Wifi}
          inactiveIcon={WifiOff}
        />
        <ProtectionRow
          label="Prozesse geschuetzt"
          active={status.processProtectionActive}
          activeLabel="Aktiv - Job Object ueberwacht Prozesse"
          inactiveLabel="Inaktiv"
          activeIcon={Lock}
          inactiveIcon={Unlock}
        />
        <ProtectionRow
          label="Dateisystem hart geschuetzt"
          active={status.filesystemProtectionActive}
          activeLabel="Aktiv - AppContainer und ACLs aktiv"
          inactiveLabel="Nicht aktiv"
          activeIcon={FolderLock}
          inactiveIcon={FolderOpen}
        />
        <OpenClawPathRow path={status.openclawPath} />
      </div>

      {error && (
        <div
          style={{
            display: "flex",
            gap: 10,
            padding: "14px 16px",
            background: "#FEF2F2",
            border: "1px solid #FECACA",
            borderRadius: 10,
            marginBottom: 16,
          }}
        >
          <AlertTriangle size={18} color="#DC2626" style={{ flexShrink: 0, marginTop: 1 }} />
          <div>
            <p style={{ fontSize: 14, fontWeight: 600, color: "#991B1B", margin: 0 }}>Fehler</p>
            <p style={{ fontSize: 13, color: "#991B1B", margin: 0, marginTop: 3 }}>{error}</p>
          </div>
        </div>
      )}

      {hasNoRules && !status.running && (
        <div
          style={{
            display: "flex",
            gap: 10,
            padding: "14px 16px",
            background: "#FEF3C7",
            border: "1px solid #FDE68A",
            borderRadius: 10,
            marginBottom: 16,
          }}
        >
          <AlertTriangle size={18} color="#D97706" style={{ flexShrink: 0, marginTop: 1 }} />
          <div>
            <p style={{ fontSize: 14, fontWeight: 600, color: "#92400E", margin: 0 }}>Noch keine Regeln definiert</p>
            <p style={{ fontSize: 13, color: "#92400E", margin: 0, marginTop: 3 }}>
              Fuege zuerst mindestens eine erlaubte App, einen Ordner oder eine Domain hinzu.
            </p>
          </div>
        </div>
      )}

      <div
        style={{
          padding: "14px 16px",
          background: "#F8FAFC",
          border: "1px solid #E2E8F0",
          borderRadius: 8,
          fontSize: 13,
          color: "#64748B",
        }}
      >
        <p style={{ fontWeight: 600, color: "#475569", margin: "0 0 6px" }}>Wie es funktioniert:</p>
        <ul style={{ margin: 0, paddingLeft: 18, lineHeight: 1.7 }}>
          <li>CrabCage startet einen lokalen HTTP-Proxy auf Port 18080</li>
          <li>OpenClaw erhaelt automatisch <code>HTTP_PROXY=127.0.0.1:18080</code></li>
          <li>Alle Netzwerkzugriffe werden gegen deine Domain-Whitelist geprueft</li>
          <li>Nicht erlaubte Domains erhalten eine 403-Antwort</li>
          <li>OpenClaw laeuft im AppContainer und sieht nur explizit freigegebene Dateipfade</li>
        </ul>
      </div>
    </div>
  );
}
