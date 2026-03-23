import { useEffect, useState } from "react";
import { Play, Square, ShieldCheck, AlertTriangle, Wifi, WifiOff, Info, Lock, Unlock } from "lucide-react";
import { useConfigStore } from "../store/configStore";
import { useSessionStore } from "../store/sessionStore";
import { SessionStatusBadge } from "../components/ui/StatusBadge";
import { OpenClawSetup } from "../components/ui/OpenClawSetup";
import { ConfirmDialog } from "../components/ui/ConfirmDialog";

function ProxyStatusRow({ active }: { active: boolean }) {
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
      {active ? (
        <Wifi size={15} color="#16A34A" />
      ) : (
        <WifiOff size={15} color="#94A3B8" />
      )}
      <span style={{ color: active ? "#15803D" : "#64748B", fontWeight: 500 }}>
        Domain-Proxy
      </span>
      <span style={{ color: active ? "#16A34A" : "#94A3B8", marginLeft: "auto", fontSize: 12 }}>
        {active ? "Aktiv – Port 18080" : "Inaktiv"}
      </span>
    </div>
  );
}

function SandboxStatusRow({ active }: { active: boolean }) {
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
      {active ? (
        <Lock size={15} color="#16A34A" />
      ) : (
        <Unlock size={15} color="#94A3B8" />
      )}
      <span style={{ color: active ? "#15803D" : "#64748B", fontWeight: 500 }}>
        Job Object Sandbox
      </span>
      <span style={{ color: active ? "#16A34A" : "#94A3B8", marginLeft: "auto", fontSize: 12 }}>
        {active ? "Aktiv – Prozesskontrolle aktiv" : "Inaktiv"}
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
    fetchStatus();
  }, [fetchStatus]);

  // Only treat as "not ready" once the config has been loaded from disk.
  // This prevents the setup card from flashing before fetchConfig completes.
  const openclawReady = !initialized || Boolean(config.openclawPath);
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
          onConfirm={() => { setConfirmStop(false); stopSession(); }}
          onCancel={() => setConfirmStop(false)}
        />
      )}
      <div style={{ marginBottom: 28 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: "0 0 6px" }}>
          Session starten
        </h1>
        <p style={{ fontSize: 14, color: "#64748B", margin: 0 }}>
          OpenClaw läuft nur innerhalb deiner Sicherheitsgrenzen.
          Alle Netzwerkzugriffe werden über den lokalen Proxy geleitet.
        </p>
      </div>

      {/* Setup flow when OpenClaw not yet configured */}
      {!openclawReady && (
        <div style={{ marginBottom: 20 }}>
          <OpenClawSetup onDone={() => fetchConfig()} />
        </div>
      )}

      {/* Main control card */}
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

        {/* Shield icon */}
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
          <p style={{ fontSize: 12, color: "#94A3B8", margin: "0 0 20px", fontFamily: "monospace" }}>
            PID {status.pid}
          </p>
        )}
        {!status.running && <div style={{ marginBottom: 20 }} />}

        {/* Start / Stop button */}
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
            {loading ? "Wird gestoppt …" : "Session beenden"}
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
            {loading ? "Wird gestartet …" : "OpenClaw starten"}
          </button>
        )}
      </div>

      {/* Status details */}
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
        <ProxyStatusRow active={status.proxyActive} />
        <SandboxStatusRow active={status.sandboxActive} />
        <OpenClawPathRow path={status.openclawPath} />
      </div>

      {/* Error display */}
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

      {/* Warning: no rules */}
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
            <p style={{ fontSize: 14, fontWeight: 600, color: "#92400E", margin: 0 }}>
              Noch keine Regeln definiert
            </p>
            <p style={{ fontSize: 13, color: "#92400E", margin: 0, marginTop: 3 }}>
              Füge zuerst mindestens eine erlaubte App, einen Ordner oder eine Domain hinzu.
            </p>
          </div>
        </div>
      )}

      {/* How it works info */}
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
          <li>CrabCage startet einen lokalen HTTP-Proxy (Port 18080)</li>
          <li>OpenClaw erhält automatisch <code>HTTP_PROXY=127.0.0.1:18080</code></li>
          <li>Alle Netzwerkzugriffe werden gegen deine Domain-Whitelist geprüft</li>
          <li>Nicht erlaubte Domains erhalten eine 403-Antwort</li>
        </ul>
      </div>
    </div>
  );
}
