import { useState } from "react";
import { Trash2 } from "lucide-react";
import { ConfirmDialog } from "./ConfirmDialog";

interface ResourceItem {
  id: string;
  label: string;
  sublabel?: string;
}

interface ResourceListProps {
  items: ResourceItem[];
  onRemove: (id: string) => void;
  emptyMessage: string;
}

export function ResourceList({ items, onRemove, emptyMessage }: ResourceListProps) {
  const [pendingId, setPendingId] = useState<string | null>(null);

  const pendingItem = items.find((i) => i.id === pendingId);

  if (items.length === 0) {
    return (
      <div
        style={{
          padding: "20px 16px",
          textAlign: "center",
          color: "#94A3B8",
          fontSize: 14,
          border: "1px dashed #E2E8F0",
          borderRadius: 8,
        }}
      >
        {emptyMessage}
      </div>
    );
  }

  return (
    <>
      {pendingItem && (
        <ConfirmDialog
          title="Wirklich entfernen?"
          message={`"${pendingItem.label}" wird aus der Erlaubnisliste gelöscht.`}
          confirmLabel="Entfernen"
          onConfirm={() => {
            onRemove(pendingId!);
            setPendingId(null);
          }}
          onCancel={() => setPendingId(null)}
        />
      )}

      <div className="flex flex-col gap-2">
        {items.map((item) => (
          <div
            key={item.id}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "10px 14px",
              background: "white",
              border: "1px solid #E2E8F0",
              borderRadius: 8,
            }}
          >
            <div>
              <p style={{ fontSize: 14, color: "#0F172A", margin: 0, fontWeight: 500 }}>{item.label}</p>
              {item.sublabel && (
                <p style={{ fontSize: 12, color: "#64748B", margin: 0, marginTop: 2 }}>{item.sublabel}</p>
              )}
            </div>
            <button
              onClick={() => setPendingId(item.id)}
              title="Entfernen"
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: "4px 6px",
                color: "#94A3B8",
                borderRadius: 6,
                display: "flex",
                alignItems: "center",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#DC2626")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "#94A3B8")}
            >
              <Trash2 size={15} />
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
