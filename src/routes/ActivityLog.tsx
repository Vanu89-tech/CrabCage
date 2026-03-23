import { useEffect } from "react";
import { useAuditStore } from "../store/auditStore";
import { StatusBadge } from "../components/ui/StatusBadge";
import { ScrollText } from "lucide-react";

function formatTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function ActivityLog() {
  const { events, fetchEvents } = useAuditStore();

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  return (
    <div style={{ maxWidth: 800 }}>
      <div style={{ marginBottom: 28 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
          <ScrollText size={20} color="#4F46E5" />
          <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: 0 }}>
            Aktivitätsprotokoll
          </h1>
        </div>
        <p style={{ fontSize: 14, color: "#64748B", margin: 0 }}>
          Jede Aktion von OpenClaw wird hier aufgezeichnet.
        </p>
      </div>

      {/* Legend */}
      <div
        style={{
          display: "flex",
          gap: 12,
          marginBottom: 20,
          padding: "12px 16px",
          background: "white",
          borderRadius: 10,
          border: "1px solid #E2E8F0",
          flexWrap: "wrap",
        }}
      >
        <span style={{ fontSize: 12, color: "#64748B", fontWeight: 500 }}>Legende:</span>
        <StatusBadge result="allowed" />
        <StatusBadge result="blocked" />
        <StatusBadge result="confirmed" />
      </div>

      {/* Event list */}
      <div
        style={{
          background: "white",
          border: "1px solid #E2E8F0",
          borderRadius: 12,
          overflow: "hidden",
        }}
      >
        {events.length === 0 ? (
          <div style={{ padding: "40px 24px", textAlign: "center", color: "#94A3B8", fontSize: 14 }}>
            Noch keine Aktivitäten aufgezeichnet.
          </div>
        ) : (
          events.map((event, idx) => (
            <div
              key={event.id}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 16,
                padding: "14px 20px",
                borderBottom: idx < events.length - 1 ? "1px solid #F1F5F9" : "none",
              }}
            >
              {/* Timestamp */}
              <div style={{ minWidth: 140 }}>
                <p style={{ fontSize: 12, color: "#94A3B8", margin: 0, fontFamily: "monospace" }}>
                  {formatTime(event.timestamp)}
                </p>
              </div>

              {/* Action + resource */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <p style={{ fontSize: 13, fontWeight: 500, color: "#0F172A", margin: 0 }}>
                  {event.action}
                </p>
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
                {event.details && (
                  <p style={{ fontSize: 11, color: "#94A3B8", margin: 0, marginTop: 2 }}>
                    {event.details}
                  </p>
                )}
              </div>

              {/* Status */}
              <StatusBadge result={event.result} />
            </div>
          ))
        )}
      </div>

      <p style={{ fontSize: 12, color: "#94A3B8", marginTop: 12, textAlign: "right" }}>
        {events.length} Einträge gespeichert · maximal 500
      </p>
    </div>
  );
}
