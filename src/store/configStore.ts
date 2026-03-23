import { create } from "zustand";
import type { CrabCageConfig, AllowedApp, AllowedPath, AllowedDomain } from "../lib/types";
import { loadConfig, saveConfig } from "../lib/tauri";

interface ConfigStore {
  config: CrabCageConfig;
  loading: boolean;
  initialized: boolean;

  fetchConfig: () => Promise<void>;
  addApp: (app: Omit<AllowedApp, "id" | "addedAt">) => Promise<void>;
  removeApp: (id: string) => Promise<void>;
  addPath: (path: Omit<AllowedPath, "id" | "addedAt">) => Promise<void>;
  removePath: (id: string) => Promise<void>;
  addDomain: (domain: Omit<AllowedDomain, "id" | "addedAt">) => Promise<void>;
  removeDomain: (id: string) => Promise<void>;
  completeOnboarding: () => Promise<void>;
  setOpenclawPath: (path: string) => Promise<void>;
}

function makeId(): string {
  return crypto.randomUUID();
}

const emptyConfig: CrabCageConfig = {
  allowedApps: [],
  allowedPaths: [],
  allowedDomains: [],
  onboardingComplete: false,
};

export const useConfigStore = create<ConfigStore>((set, get) => ({
  config: emptyConfig,
  loading: false,
  initialized: false,

  fetchConfig: async () => {
    set({ loading: true });
    try {
      const config = await loadConfig();
      set({ config, initialized: true });
    } catch {
      set({ config: emptyConfig, initialized: true });
    } finally {
      set({ loading: false });
    }
  },

  addApp: async (app) => {
    const newApp: AllowedApp = {
      ...app,
      id: makeId(),
      addedAt: new Date().toISOString(),
    };
    const config = { ...get().config, allowedApps: [...get().config.allowedApps, newApp] };
    set({ config });
    await saveConfig(config);
  },

  removeApp: async (id) => {
    const config = { ...get().config, allowedApps: get().config.allowedApps.filter((a) => a.id !== id) };
    set({ config });
    await saveConfig(config);
  },

  addPath: async (pathItem) => {
    const newPath: AllowedPath = {
      ...pathItem,
      id: makeId(),
      addedAt: new Date().toISOString(),
    };
    const config = { ...get().config, allowedPaths: [...get().config.allowedPaths, newPath] };
    set({ config });
    await saveConfig(config);
  },

  removePath: async (id) => {
    const config = { ...get().config, allowedPaths: get().config.allowedPaths.filter((p) => p.id !== id) };
    set({ config });
    await saveConfig(config);
  },

  addDomain: async (domainItem) => {
    const newDomain: AllowedDomain = {
      ...domainItem,
      id: makeId(),
      addedAt: new Date().toISOString(),
    };
    const config = { ...get().config, allowedDomains: [...get().config.allowedDomains, newDomain] };
    set({ config });
    await saveConfig(config);
  },

  removeDomain: async (id) => {
    const config = { ...get().config, allowedDomains: get().config.allowedDomains.filter((d) => d.id !== id) };
    set({ config });
    await saveConfig(config);
  },

  completeOnboarding: async () => {
    const config = { ...get().config, onboardingComplete: true };
    set({ config });
    await saveConfig(config);
  },

  setOpenclawPath: async (path) => {
    const config = { ...get().config, openclawPath: path };
    set({ config });
    await saveConfig(config);
  },
}));
