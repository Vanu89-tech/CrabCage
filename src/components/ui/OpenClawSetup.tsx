import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  CheckCircle, XCircle, Download, FolderOpen,
  Loader, ChevronRight, AlertTriangle,
} from "lucide-react";
import { checkEnvironment, installOpenClaw, validateOpenClawPath } from "../../lib/tauri";
import { useConfigStore } from "../../store/configStore";
import type { EnvironmentStatus, InstallProgressEvent } from "../../lib/types";

type Phase = "checking" | "idle" | "installing" | "manual" | "done";

interface StatusRowProps {
  label: string;
  ok: boolean | null;
  detail?: string;
}

function StatusRow({ label, ok, detail }: StatusRowProps) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "9px 14px",
        background: ok === null ? "#F8FAFC" : ok ? "#F0FDF4" : "#FEF2F2",
        border: `1px solid ${ok === null ? "#E2E8F0" : ok ? "#BBF7D0" : "#FECACA"}`,
        borderRadius: 8,
        fontSize: 13,
      }}
    >
      {ok === null ? (
        <Loader size={15} color="#94A3B8" style={{ animation: "spin 1s linear infinite" }} />
      ) : ok ? (
        <CheckCircle size={15} color="#16A34A" />
      ) : (
        <XCircle size={15} color="#DC2626" />
      )}
      <span style={{ fontWeight: 500, color: ok === null ? "#64748B" : ok ? "#15803D" : "#991B1B" }}>
        {label}
      </span>
      {detail && (
        <span style={{ marginLeft: "auto", color: "#94A3B8", fontSize: 12, fontFamily: "monospace" }}>
          {detail}
        </span>
      )}
    </div>
  );
}

interface Props {
  onDone?: (path: string) => void;
}

export function OpenClawSetup({ onDone }: Props) {
  const [phase, setPhase] = useState<Phase>("checking");
  const [env, setEnv] = useState<EnvironmentStatus | null>(null);
  const [lines, setLines] = useState<InstallProgressEvent[]>([]);
  const [manualPath, setManualPath] = useState("");
  const [validating, setValidating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const { setOpenclawPath } = useConfigStore();

  // Auto-scroll terminal output
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [lines]);

  useEffect(() => {
    checkEnvironment()
      .then((status) => {
        setEnv(status);
        if (status.openclawInstalled && status.openclawPath) {
          // Already installed – save path and finish
          setOpenclawPath(status.openclawPath).then(() => {
            onDone?.(status.openclawPath!);
            setPhase("done");
          });
        } else {
          setPhase("idle");
        }
      })
      .catch(() => setPhase("idle"));

    return () => {
      unlistenRef.current?.();
    };
  }, []);

  const handleInstall = async () => {
    setPhase("installing");
    setLines([]);
    setError(null);

    // Subscribe to streaming progress
    unlistenRef.current = await listen<InstallProgressEvent>("install_progress", (e) => {
      setLines((prev) => [...prev, e.payload]);
    });

    try {
      const path = await installOpenClaw();
      await setOpenclawPath(path);
      setPhase("done");
      onDone?.(path);
    } catch (e) {
      setError(e as string);
      setPhase("idle");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
    }
  };

  const handleValidateManual = async () => {
    if (!manualPath.trim()) return;
    setValidating(true);
    setError(null);
    try {
      await validateOpenClawPath(manualPath.trim());
      await setOpenclawPath(manualPath.trim());
      setPhase("done");
      onDone?.(manualPath.trim());
    } catch (e) {
      setError(e as string);
    } finally {
      setValidating(false);
    }
  };

  const lineColor: Record<string, string> = {
    command: "#818CF8",
    stdout: "#E2E8F0",
    stderr: "#FCA5A5",
    success: "#86EFAC",
    error: "#FCA5A5",
  };

  // ── Done ──────────────────────────────────────────────────────────────────
  if (phase === "done") {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "16px 20px",
          background: "#F0FDF4",
          border: "1px solid #BBF7D0",
          borderRadius: 10,
        }}
      >
        <CheckCircle size={22} color="#16A34A" />
        <div>
          <p style={{ fontSize: 14, fontWeight: 600, color: "#15803D", margin: 0 }}>
            OpenClaw ist eingerichtet
          </p>
          <p style={{ fontSize: 12, color: "#16A34A", margin: 0, marginTop: 2 }}>
            Du kannst jetzt eine Session starten.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        background: "white",
        border: "1px solid #E2E8F0",
        borderRadius: 12,
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "16px 20px",
          borderBottom: "1px solid #F1F5F9",
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        <Download size={18} color="#4F46E5" />
        <div>
          <p style={{ fontSize: 14, fontWeight: 600, color: "#0F172A", margin: 0 }}>
            OpenClaw einrichten
          </p>
          <p style={{ fontSize: 12, color: "#64748B", margin: 0 }}>
            Installiere OpenClaw oder gib den Pfad an.
          </p>
        </div>
      </div>

      <div style={{ padding: "16px 20px" }}>
        {/* Environment check */}
        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
          <StatusRow
            label="Node.js"
            ok={phase === "checking" ? null : env?.nodeInstalled ?? false}
            detail={env?.nodeVersion ?? undefined}
          />
          <StatusRow
            label="openclaw"
            ok={phase === "checking" ? null : env?.openclawInstalled ?? false}
            detail={env?.openclawVersion ?? undefined}
          />
        </div>

        {/* Node.js missing warning */}
        {phase !== "checking" && !env?.nodeInstalled && (
          <div
            style={{
              display: "flex",
              gap: 8,
              padding: "12px 14px",
              background: "#FEF3C7",
              border: "1px solid #FDE68A",
              borderRadius: 8,
              fontSize: 13,
              color: "#92400E",
              marginBottom: 14,
            }}
          >
            <AlertTriangle size={16} style={{ flexShrink: 0, marginTop: 1 }} color="#D97706" />
            <span>
              Node.js ist erforderlich.{" "}
              <a
                href="https://nodejs.org"
                target="_blank"
                rel="noreferrer"
                style={{ color: "#D97706", fontWeight: 600 }}
              >
                nodejs.org →
              </a>
            </span>
          </div>
        )}

        {/* Action buttons (idle) */}
        {phase === "idle" && env?.nodeInstalled && (
          <div style={{ display: "flex", gap: 8, marginBottom: lines.length ? 12 : 0 }}>
            <button
              onClick={handleInstall}
              style={{
                flex: 1,
                padding: "10px 16px",
                background: "#4F46E5",
                color: "white",
                border: "none",
                borderRadius: 8,
                fontSize: 13,
                fontWeight: 600,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 6,
              }}
            >
              <Download size={14} />
              openclaw installieren
            </button>
            <button
              onClick={() => setPhase("manual")}
              style={{
                flex: 1,
                padding: "10px 16px",
                background: "white",
                color: "#475569",
                border: "1px solid #E2E8F0",
                borderRadius: 8,
                fontSize: 13,
                fontWeight: 500,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 6,
              }}
            >
              <FolderOpen size={14} />
              Pfad angeben
            </button>
          </div>
        )}

        {/* Manual path input */}
        {phase === "manual" && (
          <div style={{ marginBottom: 12 }}>
            <p style={{ fontSize: 12, color: "#64748B", margin: "0 0 8px" }}>
              Gib den vollständigen Pfad zu deiner openclaw-Installation ein:
            </p>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                type="text"
                value={manualPath}
                onChange={(e) => setManualPath(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleValidateManual()}
                placeholder="z.B. /usr/local/bin/openclaw oder openclaw"
                autoFocus
                style={{
                  flex: 1,
                  padding: "9px 12px",
                  border: "1px solid #E2E8F0",
                  borderRadius: 8,
                  fontSize: 13,
                  fontFamily: "monospace",
                  outline: "none",
                  background: "white",
                  color: "#0F172A",
                }}
              />
              <button
                onClick={handleValidateManual}
                disabled={validating || !manualPath.trim()}
                style={{
                  padding: "9px 14px",
                  background: "#4F46E5",
                  color: "white",
                  border: "none",
                  borderRadius: 8,
                  fontSize: 13,
                  fontWeight: 600,
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  opacity: validating || !manualPath.trim() ? 0.6 : 1,
                }}
              >
                {validating ? <Loader size={14} /> : <ChevronRight size={14} />}
                Prüfen
              </button>
            </div>
            <button
              onClick={() => { setPhase("idle"); setError(null); }}
              style={{
                marginTop: 6,
                background: "none",
                border: "none",
                fontSize: 12,
                color: "#94A3B8",
                cursor: "pointer",
                padding: 0,
              }}
            >
              ← zurück
            </button>
          </div>
        )}

        {/* Install progress terminal */}
        {(phase === "installing" || lines.length > 0) && (
          <div
            ref={logRef}
            style={{
              background: "#0F172A",
              borderRadius: 8,
              padding: "12px 14px",
              fontFamily: "monospace",
              fontSize: 12,
              lineHeight: 1.6,
              maxHeight: 180,
              overflowY: "auto",
              marginBottom: 8,
            }}
          >
            {lines.length === 0 && (
              <span style={{ color: "#475569" }}>Warte auf npm…</span>
            )}
            {lines.map((l, i) => (
              <div key={i} style={{ color: lineColor[l.type] ?? "#E2E8F0" }}>
                {l.line}
              </div>
            ))}
            {phase === "installing" && (
              <span style={{ color: "#475569", animation: "pulse 1s infinite" }}>▌</span>
            )}
          </div>
        )}

        {/* Error */}
        {error && (
          <div
            style={{
              padding: "10px 14px",
              background: "#FEF2F2",
              border: "1px solid #FECACA",
              borderRadius: 8,
              fontSize: 13,
              color: "#991B1B",
              marginTop: 8,
            }}
          >
            {error}
          </div>
        )}
      </div>
    </div>
  );
}
