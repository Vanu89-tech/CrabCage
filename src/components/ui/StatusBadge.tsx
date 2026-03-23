import type { AuditResult } from "../../lib/types";

const config: Record<AuditResult, { label: string; bg: string; color: string; dot: string }> = {
  allowed: {
    label: "Erlaubt",
    bg: "#DCFCE7",
    color: "#15803D",
    dot: "#16A34A",
  },
  blocked: {
    label: "Blockiert",
    bg: "#FEE2E2",
    color: "#B91C1C",
    dot: "#DC2626",
  },
  confirmed: {
    label: "Bestätigt",
    bg: "#FEF3C7",
    color: "#92400E",
    dot: "#D97706",
  },
};

interface StatusBadgeProps {
  result: AuditResult;
}

export function StatusBadge({ result }: StatusBadgeProps) {
  const { label, bg, color, dot } = config[result];
  return (
    <span
      style={{ background: bg, color, fontSize: 12, fontWeight: 600, borderRadius: 6, padding: "3px 9px" }}
      className="inline-flex items-center gap-1.5"
    >
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: dot, display: "inline-block" }} />
      {label}
    </span>
  );
}

interface SessionStatusBadgeProps {
  active: boolean;
}

export function SessionStatusBadge({ active }: SessionStatusBadgeProps) {
  return (
    <span
      style={{
        background: active ? "#DCFCE7" : "#F1F5F9",
        color: active ? "#15803D" : "#64748B",
        fontSize: 13,
        fontWeight: 600,
        borderRadius: 8,
        padding: "5px 12px",
      }}
      className="inline-flex items-center gap-2"
    >
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          background: active ? "#16A34A" : "#94A3B8",
          display: "inline-block",
          animation: active ? "pulse 2s infinite" : "none",
        }}
      />
      {active ? "Geschützt & aktiv" : "Inaktiv"}
    </span>
  );
}
