import { useState } from "react";
import { Plus } from "lucide-react";

interface AddItemInputProps {
  placeholder: string;
  onAdd: (value: string) => void;
  disabled?: boolean;
}

export function AddItemInput({ placeholder, onAdd, disabled }: AddItemInputProps) {
  const [value, setValue] = useState("");

  const handleSubmit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onAdd(trimmed);
    setValue("");
  };

  return (
    <div className="flex gap-2">
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
        placeholder={placeholder}
        disabled={disabled}
        style={{
          flex: 1,
          padding: "9px 14px",
          border: "1px solid #E2E8F0",
          borderRadius: 8,
          fontSize: 14,
          outline: "none",
          background: "white",
          color: "#0F172A",
        }}
      />
      <button
        onClick={handleSubmit}
        disabled={disabled || !value.trim()}
        style={{
          padding: "9px 16px",
          background: "#4F46E5",
          color: "white",
          border: "none",
          borderRadius: 8,
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: 6,
          fontSize: 14,
          fontWeight: 500,
          opacity: disabled || !value.trim() ? 0.5 : 1,
        }}
      >
        <Plus size={16} />
        Hinzufügen
      </button>
    </div>
  );
}
