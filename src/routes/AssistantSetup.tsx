import { useState } from "react";
import { MessageCircle, Smartphone, ShieldCheck, ArrowRight, ExternalLink, Loader2, CheckCircle2 } from "lucide-react";
import { launchOpenClawAssistant } from "../lib/tauri";
import { useConfigStore } from "../store/configStore";

type AssistantAction = "onboard" | "configure" | "channels_login" | "dashboard";

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        background: "white",
        border: "1px solid #E2E8F0",
        borderRadius: 14,
        padding: 24,
      }}
    >
      {children}
    </div>
  );
}

function ActionButton({
  title,
  description,
  action,
  loadingAction,
  onRun,
}: {
  title: string;
  description: string;
  action: AssistantAction;
  loadingAction: AssistantAction | null;
  onRun: (action: AssistantAction) => void;
}) {
  const busy = loadingAction === action;

  return (
    <button
      onClick={() => onRun(action)}
      disabled={busy}
      style={{
        width: "100%",
        textAlign: "left",
        border: "1px solid #E2E8F0",
        borderRadius: 12,
        background: "#F8FAFC",
        padding: "16px 18px",
        cursor: busy ? "wait" : "pointer",
        display: "flex",
        alignItems: "center",
        gap: 14,
      }}
    >
      <div
        style={{
          width: 40,
          height: 40,
          borderRadius: 10,
          background: "#E0E7FF",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
        }}
      >
        {busy ? <Loader2 size={18} color="#4F46E5" className="animate-spin" /> : <ArrowRight size={18} color="#4F46E5" />}
      </div>
      <div style={{ flex: 1 }}>
        <p style={{ margin: 0, fontSize: 14, fontWeight: 700, color: "#0F172A" }}>{title}</p>
        <p style={{ margin: "4px 0 0", fontSize: 12, color: "#64748B", lineHeight: 1.5 }}>{description}</p>
      </div>
    </button>
  );
}

export function AssistantSetup() {
  const openclawPath = useConfigStore((state) => state.config.openclawPath);
  const [loadingAction, setLoadingAction] = useState<AssistantAction | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runAction = async (action: AssistantAction) => {
    setLoadingAction(action);
    setError(null);
    setMessage(null);

    try {
      const result = await launchOpenClawAssistant(action);
      setMessage(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingAction(null);
    }
  };

  return (
    <div style={{ maxWidth: 860 }}>
      <div style={{ marginBottom: 28 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 8 }}>
          <div
            style={{
              width: 42,
              height: 42,
              borderRadius: 12,
              background: "#DCFCE7",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <MessageCircle size={20} color="#15803D" />
          </div>
          <div>
            <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: 0 }}>Personal Assistant</h1>
            <p style={{ fontSize: 14, color: "#64748B", margin: "4px 0 0" }}>
              Richte OpenClaw als immer erreichbaren Assistenten für WhatsApp und andere Messenger ein.
            </p>
          </div>
        </div>
      </div>

      <div style={{ display: "grid", gap: 16, marginBottom: 20 }}>
        <Card>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 16 }}>
            <div>
              <p style={{ margin: 0, fontSize: 15, fontWeight: 700, color: "#0F172A" }}>OpenClaw Pfad</p>
              <p style={{ margin: "6px 0 0", fontSize: 12, color: openclawPath ? "#334155" : "#B91C1C", fontFamily: "monospace" }}>
                {openclawPath ?? "Noch nicht eingerichtet"}
              </p>
            </div>
            <a
              href="https://docs.openclaw.ai/cli"
              target="_blank"
              rel="noreferrer"
              style={{
                color: "#4F46E5",
                textDecoration: "none",
                fontSize: 13,
                fontWeight: 600,
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
                whiteSpace: "nowrap",
              }}
            >
              CLI-Doku <ExternalLink size={14} />
            </a>
          </div>
        </Card>

        <Card>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
            <Smartphone size={18} color="#4F46E5" />
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 700, color: "#0F172A" }}>Empfohlener WhatsApp-Flow</h2>
          </div>
          <div style={{ display: "grid", gap: 10 }}>
            <ActionButton
              title="1. Onboarding Wizard starten"
              description="Startet den interaktiven OpenClaw-Onboarding-Flow für Gateway, Workspace und Skills."
              action="onboard"
              loadingAction={loadingAction}
              onRun={runAction}
            />
            <ActionButton
              title="2. Persönlichen Assistenten konfigurieren"
              description="Öffnet den Konfigurationswizard für Credentials, Channels und Standardverhalten."
              action="configure"
              loadingAction={loadingAction}
              onRun={runAction}
            />
            <ActionButton
              title="3. WhatsApp verbinden"
              description="Startet 'openclaw channels login', damit du WhatsApp oder weitere Messenger verbinden kannst."
              action="channels_login"
              loadingAction={loadingAction}
              onRun={runAction}
            />
            <ActionButton
              title="4. OpenClaw Dashboard öffnen"
              description="Öffnet das Control UI mit deinem aktuellen Token, falls du den Assistant-Zustand prüfen willst."
              action="dashboard"
              loadingAction={loadingAction}
              onRun={runAction}
            />
          </div>
        </Card>

        <Card>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
            <ShieldCheck size={18} color="#15803D" />
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 700, color: "#0F172A" }}>Für Non-Tech-User am besten geeignet</h2>
          </div>
          <ul style={{ margin: 0, paddingLeft: 18, color: "#475569", fontSize: 13, lineHeight: 1.8 }}>
            <li>Per WhatsApp kurze Aufgaben starten: "Finde meine letzte Rechnung"</li>
            <li>Ergebnisse und Statusmeldungen direkt im Chat erhalten</li>
            <li>Rückfragen oder Bestätigungen beantworten, statt Terminalbefehle zu lernen</li>
            <li>CrabCage behält die lokalen Grenzen für Dateien, Apps und Websites bei</li>
          </ul>
        </Card>
      </div>

      {message && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "14px 16px",
            background: "#F0FDF4",
            border: "1px solid #BBF7D0",
            borderRadius: 10,
            marginBottom: 16,
          }}
        >
          <CheckCircle2 size={18} color="#16A34A" />
          <p style={{ margin: 0, color: "#166534", fontSize: 13 }}>{message}</p>
        </div>
      )}

      {error && (
        <div
          style={{
            padding: "14px 16px",
            background: "#FEF2F2",
            border: "1px solid #FECACA",
            borderRadius: 10,
            color: "#991B1B",
            fontSize: 13,
          }}
        >
          {error}
        </div>
      )}
    </div>
  );
}
