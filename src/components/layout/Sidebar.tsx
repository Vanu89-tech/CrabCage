import { NavLink } from "react-router-dom";
import { LayoutDashboard, ShieldCheck, ScrollText, Play, MessageCircleMore } from "lucide-react";

const navItems = [
  { to: "/dashboard", icon: LayoutDashboard, label: "Übersicht" },
  { to: "/permissions", icon: ShieldCheck, label: "Berechtigungen" },
  { to: "/activity", icon: ScrollText, label: "Aktivitätsprotokoll" },
  { to: "/assistant", icon: MessageCircleMore, label: "Assistant" },
  { to: "/session", icon: Play, label: "Session starten" },
];

export function Sidebar() {
  return (
    <aside
      style={{ width: 220, minWidth: 220, background: "#0F172A" }}
      className="flex flex-col h-full"
    >
      <div className="flex items-center gap-3 px-5 py-6 border-b border-white/10">
        <div
          style={{ background: "#4F46E5" }}
          className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
        >
          <ShieldCheck size={18} color="white" />
        </div>
        <span className="font-semibold text-white text-base tracking-tight">CrabCage</span>
      </div>

      <nav className="flex flex-col gap-1 p-3 flex-1">
        {navItems.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            style={({ isActive }) => ({
              display: "flex",
              alignItems: "center",
              gap: 10,
              padding: "9px 12px",
              borderRadius: 8,
              textDecoration: "none",
              fontSize: 14,
              fontWeight: isActive ? 600 : 400,
              color: isActive ? "white" : "#94A3B8",
              background: isActive ? "#1E293B" : "transparent",
              borderLeft: isActive ? "3px solid #4F46E5" : "3px solid transparent",
              transition: "all 0.15s ease",
            })}
          >
            <Icon size={16} />
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="px-5 py-4 border-t border-white/10">
        <p style={{ fontSize: 11, color: "#475569" }} className="leading-relaxed">
          Alles lokal.
          <br />
          Deine Regeln. Deine Kontrolle.
        </p>
      </div>
    </aside>
  );
}
