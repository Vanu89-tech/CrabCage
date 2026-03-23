import { useEffect } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import { Onboarding } from "./routes/Onboarding";
import { Dashboard } from "./routes/Dashboard";
import { Permissions } from "./routes/Permissions";
import { ActivityLog } from "./routes/ActivityLog";
import { SessionControl } from "./routes/SessionControl";
import { AssistantSetup } from "./routes/AssistantSetup";
import { useConfigStore } from "./store/configStore";

function AppRoutes() {
  const { config, initialized, fetchConfig } = useConfigStore();

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  if (!initialized) {
    return (
      <div
        style={{
          minHeight: "100vh",
          background: "#0F172A",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <div style={{ color: "#475569", fontSize: 14 }}>CrabCage wird geladen...</div>
      </div>
    );
  }

  if (!config.onboardingComplete) {
    return (
      <Routes>
        <Route path="/onboarding" element={<Onboarding />} />
        <Route path="*" element={<Navigate to="/onboarding" replace />} />
      </Routes>
    );
  }

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/permissions" element={<Permissions />} />
        <Route path="/activity" element={<ActivityLog />} />
        <Route path="/assistant" element={<AssistantSetup />} />
        <Route path="/session" element={<SessionControl />} />
      </Route>
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}
