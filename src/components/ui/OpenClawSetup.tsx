import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  AlertTriangle,
  CheckCircle,
  ChevronRight,
  Download,
  FolderOpen,
  Loader,
  XCircle,
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

function formatTauriError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === "object") {
    const maybeMessage = Reflect.get(error, "message");
    if (typeof maybeMessage === "string") {
      return maybeMessage;
    }

    const maybeCause = Reflect.get(error, "cause");
    if (typeof maybeCause === "string") {
      return maybeCause;
    }
  }

  return "Unbekannter Fehler bei der OpenClaw-Einrichtung.";
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
          setOpenclawPath(status.openclawPath).then(() => {
            onDone?.(status.openclawPath!);
            setPhase("done");
          });
          return;
        }

        setPhase("idle");
      })
      .catch((err) => {
        setError(formatTauriError(err));
        setPhase("idle");
      });

    return () => {
      unlistenRef.current?.();
    };
  }, [onDone, setOpenclawPath]);

  const handleInstall = async () => {
    setPhase("installing");
    setLines([]);
    setError(null);

    unlistenRef.current = await listen<InstallProgressEvent>("install_progress", (event) => {
      setLines((prev) => [...prev, event.payload]);
    });

    try {
      const path = await installOpenClaw();
      await setOpenclawPath(path);
      setPhase("done");
      onDone?.(path);
    } catch (err) {
      setError(formatTauriError(err));
      setPhase("idle");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
    }
  };

  const handleValidateManual = async () => {
    const trimmed = manualPath.trim();
    if (!trimmed) {
      return;
    }

    setValidating(true);
    setError(null);

    try {
      await validateOpenClawPath(trimmed);
      await setOpenclawPath(trimmed);
      setPhase("done");
      onDone?.(trimmed);
    } catch (err) {
      setError(formatTauriError(err));
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
          <p style={{ fontSize: 12, color: "#16A34A", margin: "2px 0 0" }}>
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
                nodejs.org {"->"}
              </a>
            </span>
          </div>
        )}

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
              onClick={() => {
                setManualPath(env?.openclawPath ?? manualPath);
                setPhase("manual");
                setError(null);
              }}
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

        {phase === "manual" && (
          <div style={{ marginBottom: 12 }}>
            <p style={{ fontSize: 12, color: "#64748B", margin: "0 0 8px" }}>
              Gib den vollstaendigen Pfad zu deiner openclaw-Installation ein:
            </p>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                type="text"
                value={manualPath}
                onChange={(event) => setManualPath(event.target.value)}
                onKeyDown={(event) => event.key === "Enter" && handleValidateManual()}
                placeholder="z.B. C:\\Users\\<Name>\\AppData\\Roaming\\npm\\openclaw.cmd"
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
                Pruefen
              </button>
            </div>
            <p style={{ fontSize: 12, color: "#94A3B8", margin: "8px 0 0" }}>
              Unter Windows ist meist die Datei mit der Endung <code>.cmd</code> korrekt.
            </p>
            <button
              onClick={() => {
                setPhase("idle");
                setError(null);
              }}
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
              {"<-"} zurueck
            </button>
          </div>
        )}

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
            {lines.length === 0 && <span style={{ color: "#475569" }}>Warte auf npm...</span>}
            {lines.map((line, index) => (
              <div key={index} style={{ color: lineColor[line.type] ?? "#E2E8F0" }}>
                {line.line}
              </div>
            ))}
            {phase === "installing" && (
              <span style={{ color: "#475569", animation: "pulse 1s infinite" }}>|</span>
            )}
          </div>
        )}

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
              whiteSpace: "pre-wrap",
            }}
          >
            {error}
          </div>
        )}
      </div>
    </div>
  );
}
