import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useAuditStore } from "./auditStore";

export interface SessionStatus {
  running: boolean;
  pid: number | null;
  proxyActive: boolean;
  openclawPath: string | null;
  sandboxActive: boolean;
}

export interface ProxyEventPayload {
  domain: string;
  action: string;
  allowed: boolean;
}

interface SessionStore {
  status: SessionStatus;
  loading: boolean;
  error: string | null;
  unlisten: UnlistenFn | null;

  fetchStatus: () => Promise<void>;
  startSession: () => Promise<void>;
  stopSession: () => Promise<void>;
  subscribeToProxyEvents: () => Promise<void>;
  unsubscribeFromProxyEvents: () => void;
}

const defaultStatus: SessionStatus = {
  running: false,
  pid: null,
  proxyActive: false,
  openclawPath: null,
  sandboxActive: false,
};

export const useSessionStore = create<SessionStore>((set, get) => ({
  status: defaultStatus,
  loading: false,
  error: null,
  unlisten: null,

  fetchStatus: async () => {
    try {
      const status = await invoke<SessionStatus>("get_session_status");
      set({ status });
    } catch {
      set({ status: defaultStatus });
    }
  },

  startSession: async () => {
    set({ loading: true, error: null });
    try {
      const status = await invoke<SessionStatus>("start_session");
      set({ status });
      // Start listening to proxy events once session is live
      await get().subscribeToProxyEvents();
    } catch (e) {
      set({ error: e as string });
    } finally {
      set({ loading: false });
    }
  },

  stopSession: async () => {
    set({ loading: true, error: null });
    try {
      const status = await invoke<SessionStatus>("stop_session");
      set({ status });
      get().unsubscribeFromProxyEvents();
    } catch (e) {
      set({ error: e as string });
    } finally {
      set({ loading: false });
    }
  },

  subscribeToProxyEvents: async () => {
    // Unsubscribe first if already listening
    get().unsubscribeFromProxyEvents();

    const unlisten = await listen<ProxyEventPayload>("proxy_event", (event) => {
      const { domain, action, allowed } = event.payload;
      useAuditStore.getState().logEvent(
        action,
        domain,
        allowed ? "allowed" : "blocked",
        allowed ? `Domain '${domain}' ist erlaubt` : `Domain '${domain}' wurde blockiert`,
      );
    });

    set({ unlisten });
  },

  unsubscribeFromProxyEvents: () => {
    const { unlisten } = get();
    if (unlisten) {
      unlisten();
      set({ unlisten: null });
    }
  },
}));
