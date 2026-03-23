import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { ShieldCheck, FolderOpen, Globe, ChevronRight } from "lucide-react";
import { useConfigStore } from "../store/configStore";

const steps = [
  {
    icon: ShieldCheck,
    iconColor: "#4F46E5",
    title: "Willkommen bei CrabCage",
    text: "Du entscheidest, worauf OpenClaw zugreifen darf. Standardmäßig ist alles gesperrt – bis du es ausdrücklich erlaubst.",
    note: null,
  },
  {
    icon: FolderOpen,
    iconColor: "#16A34A",
    title: "Du legst die Grenzen fest",
    text: "Bestimme, welche Programme, Ordner und Websites OpenClaw verwenden darf. Alles andere wird automatisch blockiert.",
    note: "Nur das, was du freigibst, ist erreichbar.",
  },
  {
    icon: Globe,
    iconColor: "#D97706",
    title: "Du behältst die Kontrolle",
    text: "Jede Aktion wird protokolliert. Bei riskanten Aktionen fragt CrabCage zuerst nach. Du kannst jederzeit Regeln ändern oder die Session stoppen.",
    note: null,
  },
];

export function Onboarding() {
  const [step, setStep] = useState(0);
  const navigate = useNavigate();
  const completeOnboarding = useConfigStore((s) => s.completeOnboarding);

  const isLast = step === steps.length - 1;
  const current = steps[step];
  const Icon = current.icon;

  const handleNext = async () => {
    if (isLast) {
      await completeOnboarding();
      navigate("/dashboard");
    } else {
      setStep(step + 1);
    }
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        background: "#0F172A",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 32,
      }}
    >
      <div
        style={{
          background: "white",
          borderRadius: 16,
          padding: "48px 40px",
          maxWidth: 460,
          width: "100%",
          textAlign: "center",
        }}
      >
        {/* Icon */}
        <div
          style={{
            width: 64,
            height: 64,
            borderRadius: 16,
            background: current.iconColor + "18",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            margin: "0 auto 24px",
          }}
        >
          <Icon size={32} color={current.iconColor} />
        </div>

        {/* Title */}
        <h1 style={{ fontSize: 22, fontWeight: 700, color: "#0F172A", margin: "0 0 12px" }}>
          {current.title}
        </h1>

        {/* Text */}
        <p style={{ fontSize: 15, color: "#475569", lineHeight: 1.6, margin: "0 0 16px" }}>
          {current.text}
        </p>

        {/* Note */}
        {current.note && (
          <div
            style={{
              background: "#F8FAFC",
              border: "1px solid #E2E8F0",
              borderRadius: 8,
              padding: "10px 14px",
              fontSize: 13,
              color: "#64748B",
              marginBottom: 16,
            }}
          >
            {current.note}
          </div>
        )}

        {/* Step dots */}
        <div style={{ display: "flex", justifyContent: "center", gap: 6, marginBottom: 28 }}>
          {steps.map((_, i) => (
            <div
              key={i}
              style={{
                width: i === step ? 20 : 6,
                height: 6,
                borderRadius: 3,
                background: i === step ? "#4F46E5" : "#E2E8F0",
                transition: "all 0.2s ease",
              }}
            />
          ))}
        </div>

        {/* Button */}
        <button
          onClick={handleNext}
          style={{
            width: "100%",
            padding: "13px 24px",
            background: "#4F46E5",
            color: "white",
            border: "none",
            borderRadius: 10,
            fontSize: 15,
            fontWeight: 600,
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 8,
          }}
        >
          {isLast ? "Jetzt starten" : "Weiter"}
          <ChevronRight size={18} />
        </button>
      </div>
    </div>
  );
}
