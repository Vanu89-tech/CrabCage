import { create } from "zustand";
import type { AuditEvent, AuditResult } from "../lib/types";
import { loadAuditLog, addAuditEvent } from "../lib/tauri";

interface AuditStore {
  events: AuditEvent[];
  loading: boolean;

  fetchEvents: () => Promise<void>;
  logEvent: (action: string, resource: string, result: AuditResult, details?: string) => Promise<void>;
}

// Seed events shown when no real events exist yet
export const SEED_EVENTS: AuditEvent[] = [
  {
    id: "seed-1",
    timestamp: new Date(Date.now() - 60_000).toISOString(),
    action: "Datei lesen",
    resource: "~/Dokumente/Notizen.txt",
    result: "allowed",
    details: "Lesezugriff auf erlaubten Pfad",
  },
  {
    id: "seed-2",
    timestamp: new Date(Date.now() - 120_000).toISOString(),
    action: "Website öffnen",
    resource: "example-blocked.com",
    result: "blocked",
    details: "Domain nicht in der Erlaubnisliste",
  },
  {
    id: "seed-3",
    timestamp: new Date(Date.now() - 300_000).toISOString(),
    action: "Datei löschen",
    resource: "~/Desktop/temp.txt",
    result: "confirmed",
    details: "Kritische Aktion – Bestätigung eingeholt",
  },
];

export const useAuditStore = create<AuditStore>((set, get) => ({
  events: [],
  loading: false,

  fetchEvents: async () => {
    set({ loading: true });
    try {
      const events = await loadAuditLog();
      set({ events: events.length > 0 ? events : SEED_EVENTS });
    } catch {
      set({ events: SEED_EVENTS });
    } finally {
      set({ loading: false });
    }
  },

  logEvent: async (action, resource, result, details) => {
    const event: AuditEvent = {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      action,
      resource,
      result,
      details,
    };
    const events = [event, ...get().events].slice(0, 500);
    set({ events });
    await addAuditEvent(event);
  },
}));
