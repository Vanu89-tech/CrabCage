import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  AppWindow,
  AlertTriangle,
  ArrowRight,
  Clock,
  FolderLock,
  FolderOpen,
  Globe,
  Lock,
  ShieldCheck,
  Wifi,
} from "lucide-react";
import { useConfigStore } from "../store/configStore";
import { useAuditStore } from "../store/auditStore";
import { useSessionStore } from "../store/sessionStore";
import { StatusBadge, SessionStatusBadge } from "../components/ui/StatusBadge";

function Card({ children, style }: { children: React.ReactNode; style?: React.CSSProperties }) {
  return (
    <div
      style={{
        background: "white",
        borderRadius: 12,
        border: "1px solid #E2E8F0",
        padding: "20px 24px",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

function StatBox({
  count,
  label,
  icon: Icon,
  color,
}: {
  count: number;
  label: string;
  icon: React.ElementType;
  color: string;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 14,
        padding: "16px 20px",
        background: color + "0D",
        border: `1px solid ${color}22`,
        borderRadius: 10,
        flex: 1,
      }}
    >
      <div style={{ background: color + "18", borderRadius: 8, padding: 8 }}>
        <Icon size={18} color={color} />
      </div>
      <div>
        <p style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: 0 }}>{count}</p>
        <p style={{ fontSize: 12, color: "#64748B", margin: 0 }}>{label}</p>
      </div>
    </div>
  );
}

function ProtectionBadge({ active, label, icon: Icon }: { active: boolean; label: string; icon: React.ElementType }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "8px 10px",
        borderRadius: 8,
        background: active ? "#F0FDF4" : "#F8FAFC",
        border: `1px solid ${active ? "#BBF7D0" : "#E2E8F0"}`,
        fontSize: 12,
        color: active ? "#15803D" : "#64748B",
        fontWeight: 600,
      }}
    >
      <Icon size={14} color={active ? "#16A34A" : "#94A3B8"} />
      {label}
    </div>
  );
}

export function Dashboard() {
  const navigate = useNavigate();
  const { config, fetchConfig } = useConfigStore();
  const { events, fetchEvents } = useAuditStore();
  const { status: sessionStatus, fetchStatus } = useSessionStore();

  useEffect(() => {
    fetchConfig();
    fetchEvents();
    fetchStatus();
  }, [fetchConfig, fetchEvents, fetchStatus]);

  const recentEvents = events.slice(0, 5);
  const isActive = sessionStatus.running;
  const openclawReady = Boolean(sessionStatus.openclawPath);

   return (
    <div style={{ maxWidth: 860 }}>
      <div style={{ marginBottom: 28 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 6 }}>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: 0 }}>Uebersicht</h1>
          <SessionStatusBadge active={isActive} />
        </div>
        <p style={{ fontSize: 14, color: "#64748B", margin: 0 }}>
          Hier siehst du den aktuellen Status deiner Sicherheitskonfiguration.
        </p>
      </div>

      {!openclawReady && (
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
            padding: "14px 16px",
            background: "#FEF2F2",
            border: "1px solid #FECACA",
            borderRadius: 10,
            marginBottom: 20,
          }}
        >
          <AlertTriangle size={18} color="#DC2626" style={{ flexShrink: 0, marginTop: 1 }} />
          <div style={{ flex: 1 }}>
            <p style={{ fontSize: 13, fontWeight: 700, color: "#991B1B", margin: 0 }}>
              OpenClaw ist nicht installiert oder nicht mehr erreichbar
            </p>
            <p style={{ fontSize: 12, color: "#991B1B", margin: "3px 0 0" }}>
              Installiere OpenClaw zuerst in CrabCage, bevor du eine Session startest.
            </p>
          </div>
          <button
            onClick={() => navigate("/session")}
            style={{
              padding: "7px 14px",
              background: "#DC2626",
              color: "white",
              border: "none",
              borderRadius: 7,
              fontSize: 12,
              fontWeight: 600,
              cursor: "pointer",
              whiteSpace: "nowrap",
            }}
          >
            Jetzt installieren {"->"}
          </button>
        </div>
      )}

      <div style={{ display: "flex", gap: 12, marginBottom: 20 }}>
        <StatBox count={config.allowedApps.length} label="Erlaubte Apps" icon={AppWindow} color="#4F46E5" />
        <StatBox count={config.allowedPaths.length} label="Erlaubte Ordner" icon={FolderOpen} color="#16A34A" />
        <StatBox count={config.allowedDomains.length} label="Erlaubte Domains" icon={Globe} color="#0EA5E9" />
      </div>

      <Card style={{ marginBottom: 20, borderLeft: "4px solid #4F46E5" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <ShieldCheck size={22} color="#4F46E5" />
          <div style={{ flex: 1 }}>
            <p style={{ fontSize: 15, fontWeight: 600, color: "#0F172A", margin: 0 }}>Sicherheitsmodus: Default-Deny</p>
            <p style={{ fontSize: 13, color: "#64748B", margin: 0, marginTop: 2 }}>
              Alles, was nicht explizit erlaubt ist, wird automatisch blockiert.
            </p>
          </div>
          <button
            onClick={() => navigate("/permissions")}
            style={{
              background: "#4F46E5",
              color: "white",
              border: "none",
              borderRadius: 8,
              padding: "8px 14px",
              fontSize: 13,
              fontWeight: 500,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            Regeln verwalten <ArrowRight size={14} />
          </button>
        </div>
      </Card>

      <Card style={{ marginBottom: 20 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 14 }}>
          <h2 style={{ fontSize: 15, fontWeight: 600, color: "#0F172A", margin: 0 }}>Aktive Schutzebenen</h2>
          <button
            onClick={() => navigate("/session")}
            style={{ background: "none", border: "none", fontSize: 13, color: "#4F46E5", cursor: "pointer", fontWeight: 500 }}
          >
            Session oeffnen {"->"}
          </button>
        </div>
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          <ProtectionBadge active={sessionStatus.networkProtectionActive} label="Netzwerk" icon={Wifi} />
          <ProtectionBadge active={sessionStatus.processProtectionActive} label="Prozesse" icon={Lock} />
          <ProtectionBadge active={sessionStatus.filesystemProtectionActive} label="Dateisystem" icon={FolderLock} />
        </div>
      </Card>

      <Card>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Clock size={16} color="#64748B" />
            <h2 style={{ fontSize: 15, fontWeight: 600, color: "#0F172A", margin: 0 }}>Letzte Aktivitaeten</h2>
          </div>
          <button
            onClick={() => navigate("/activity")}
            style={{ background: "none", border: "none", fontSize: 13, color: "#4F46E5", cursor: "pointer", fontWeight: 500 }}
          >
            Alle anzeigen {"->"}
          </button>
        </div>

        {recentEvents.length === 0 ? (
          <p style={{ fontSize: 14, color: "#94A3B8", textAlign: "center", padding: "16px 0" }}>
            Noch keine Aktivitaeten aufgezeichnet.
          </p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {recentEvents.map((event) => (
              <div
                key={event.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "10px 14px",
                  background: "#F8FAFC",
                  borderRadius: 8,
                  border: "1px solid #F1F5F9",
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <p style={{ fontSize: 13, fontWeight: 500, color: "#0F172A", margin: 0 }}>{event.action}</p>
                  <p
                    style={{
                      fontSize: 12,
                      color: "#64748B",
                      margin: 0,
                      marginTop: 2,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {event.resource}
                  </p>
                </div>
                <StatusBadge result={event.result} />
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
