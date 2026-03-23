import { useState } from "react";
import { AppWindow, FolderOpen, Globe } from "lucide-react";
import { useConfigStore } from "../store/configStore";
import { AddItemInput } from "../components/ui/AddItemInput";
import { ResourceList } from "../components/ui/ResourceList";

type Tab = "apps" | "paths" | "domains";

const tabs: { id: Tab; label: string; icon: React.ElementType; color: string }[] = [
  { id: "apps", label: "Programme", icon: AppWindow, color: "#4F46E5" },
  { id: "paths", label: "Ordner & Dateien", icon: FolderOpen, color: "#16A34A" },
  { id: "domains", label: "Websites", icon: Globe, color: "#0EA5E9" },
];

function SectionHeader({ title, description }: { title: string; description: string }) {
  return (
    <div style={{ marginBottom: 16 }}>
      <h2 style={{ fontSize: 16, fontWeight: 600, color: "#0F172A", margin: "0 0 4px" }}>{title}</h2>
      <p style={{ fontSize: 13, color: "#64748B", margin: 0 }}>{description}</p>
    </div>
  );
}

export function Permissions() {
  const [activeTab, setActiveTab] = useState<Tab>("apps");
  const { config, addApp, removeApp, addPath, removePath, addDomain, removeDomain } = useConfigStore();

  return (
    <div style={{ maxWidth: 720 }}>
      <div style={{ marginBottom: 28 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: "0 0 6px" }}>
          Berechtigungen
        </h1>
        <p style={{ fontSize: 14, color: "#64748B", margin: 0 }}>
          Alles, was hier nicht steht, ist für OpenClaw gesperrt.
        </p>
      </div>

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: 4,
          background: "white",
          border: "1px solid #E2E8F0",
          borderRadius: 10,
          padding: 4,
          marginBottom: 24,
        }}
      >
        {tabs.map(({ id, label, icon: Icon, color }) => {
          const isActive = activeTab === id;
          return (
            <button
              key={id}
              onClick={() => setActiveTab(id)}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 7,
                padding: "9px 12px",
                border: "none",
                borderRadius: 7,
                background: isActive ? color : "transparent",
                color: isActive ? "white" : "#64748B",
                fontSize: 13,
                fontWeight: isActive ? 600 : 400,
                cursor: "pointer",
                transition: "all 0.15s ease",
              }}
            >
              <Icon size={15} />
              {label}
            </button>
          );
        })}
      </div>

      {/* Content */}
      <div
        style={{
          background: "white",
          border: "1px solid #E2E8F0",
          borderRadius: 12,
          padding: "24px",
        }}
      >
        {activeTab === "apps" && (
          <>
            <SectionHeader
              title="Erlaubte Programme"
              description="OpenClaw darf nur diese Anwendungen starten oder ansprechen."
            />
            <div style={{ marginBottom: 16 }}>
              <AddItemInput
                placeholder="z.B. notepad.exe oder /usr/bin/code"
                onAdd={(value) => addApp({ name: value.split(/[\\/]/).pop() ?? value, path: value })}
              />
            </div>
            <ResourceList
              items={config.allowedApps.map((a) => ({ id: a.id, label: a.name, sublabel: a.path }))}
              onRemove={removeApp}
              emptyMessage="Noch keine Programme erlaubt. Füge ein Programm hinzu, um zu beginnen."
            />
          </>
        )}

        {activeTab === "paths" && (
          <>
            <SectionHeader
              title="Erlaubte Ordner & Dateipfade"
              description="OpenClaw darf nur in diesen Verzeichnissen lesen oder schreiben."
            />
            <div style={{ marginBottom: 16 }}>
              <AddItemInput
                placeholder="z.B. C:\Dokumente\Projekt oder ~/Downloads"
                onAdd={(value) => addPath({ path: value, permissions: ["read"] })}
              />
            </div>
            <ResourceList
              items={config.allowedPaths.map((p) => ({
                id: p.id,
                label: p.path,
                sublabel: p.permissions.includes("write") ? "Lesen & Schreiben" : "Nur Lesen",
              }))}
              onRemove={removePath}
              emptyMessage="Noch keine Ordner erlaubt. Füge einen Pfad hinzu."
            />
          </>
        )}

        {activeTab === "domains" && (
          <>
            <SectionHeader
              title="Erlaubte Websites"
              description="OpenClaw darf nur diese Domains im Browser öffnen oder aufrufen."
            />
            <div style={{ marginBottom: 16 }}>
              <AddItemInput
                placeholder="z.B. wikipedia.org oder docs.python.org"
                onAdd={(value) => addDomain({ domain: value.replace(/^https?:\/\//, "").split("/")[0] })}
              />
            </div>
            <ResourceList
              items={config.allowedDomains.map((d) => ({ id: d.id, label: d.domain }))}
              onRemove={removeDomain}
              emptyMessage="Noch keine Websites erlaubt. Füge eine Domain hinzu."
            />
          </>
        )}
      </div>

      {/* Info box */}
      <div
        style={{
          marginTop: 16,
          padding: "12px 16px",
          background: "#F8FAFC",
          border: "1px solid #E2E8F0",
          borderRadius: 8,
          fontSize: 13,
          color: "#64748B",
          display: "flex",
          gap: 8,
          alignItems: "flex-start",
        }}
      >
        <span style={{ flexShrink: 0 }}>ℹ️</span>
        <span>
          Änderungen werden sofort lokal gespeichert. Sie gelten ab der nächsten OpenClaw-Session.
        </span>
      </div>
    </div>
  );
}
