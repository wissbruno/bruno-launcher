import { defineStore } from 'pinia';
import {
  isTauri,
  listInstances,
  getRunning,
  getAccounts,
  getSettings,
  setSettings,
  onProgress,
  onGameLog,
  onGameExit,
  type Instance,
  type AccountPublic,
  type Settings,
  type ProgressEvent,
} from '../api/backend';

/** Estado global do launcher no frontend, alimentado por eventos do Rust. */
export const useLauncherStore = defineStore('launcher', {
  state: () => ({
    isTauri,
    instances: [] as Instance[],
    running: new Set<string>(),
    accounts: [] as AccountPublic[],
    settings: null as Settings | null,
    /** progresso ativo por id de operação (geralmente id da instância) */
    progress: new Map<string, ProgressEvent>(),
    /** últimas linhas de log por instância */
    logs: new Map<string, string[]>(),
    initialized: false,
  }),
  getters: {
    activeAccount: (state) => state.accounts.find((a) => a.active) ?? null,
    activeProgress: (state) => [...state.progress.values()].filter((p) => !p.done),
  },
  actions: {
    async init() {
      if (this.initialized || !isTauri) return;
      this.initialized = true;

      await onProgress((e) => {
        this.progress.set(e.id, e);
        if (e.done) {
          // some da lista depois de alguns segundos, e recarrega instâncias
          setTimeout(() => {
            const current = this.progress.get(e.id);
            if (current?.done) this.progress.delete(e.id);
          }, 4000);
          this.refreshInstances();
        }
      });
      await onGameLog((e) => {
        const lines = this.logs.get(e.id) ?? [];
        lines.push(e.line);
        if (lines.length > 500) lines.splice(0, lines.length - 500);
        this.logs.set(e.id, lines);
      });
      await onGameExit((e) => {
        this.running.delete(e.id);
        const lines = this.logs.get(e.id) ?? [];
        lines.push(`--- Jogo encerrado (código ${e.code}) ---`);
        this.logs.set(e.id, lines);
        // Recarrega para refletir o playtime recém-somado no backend
        this.refreshInstances();
      });

      await Promise.all([this.refreshInstances(), this.refreshAccounts(), this.loadSettings()]);
      try {
        this.running = new Set(await getRunning());
      } catch {
        /* ignore */
      }
    },
    async refreshInstances() {
      try {
        this.instances = await listInstances();
      } catch {
        /* navegador sem Tauri */
      }
    },
    async refreshAccounts() {
      try {
        this.accounts = await getAccounts();
      } catch {
        /* navegador sem Tauri */
      }
    },
    async loadSettings() {
      try {
        this.settings = await getSettings();
      } catch {
        /* navegador sem Tauri */
      }
    },
    async saveSettings(settings: Settings) {
      this.settings = await setSettings(settings);
    },
    markRunning(id: string) {
      this.running.add(id);
      this.logs.set(id, []);
    },
  },
});
