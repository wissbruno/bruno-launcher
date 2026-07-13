/**
 * Ponte com o backend Rust (comandos Tauri).
 * No navegador (dev sem Tauri) as chamadas falham com mensagem clara.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface Instance {
  id: string;
  name: string;
  game_version: string;
  loader: 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge';
  loader_version: string | null;
  created: string;
  last_played: string | null;
  icon_url: string | null;
  modpack: string | null;
  installed: boolean;
  playtime_seconds: number;
  custom_icon: boolean;
  pinned: boolean;
  notes: string | null;
  accent_color: string | null;
}

export interface ContentFile {
  folder: string;
  filename: string;
  size: number;
}

export interface GameVersionEntry {
  id: string;
  version_type: string;
}

export interface Settings {
  memory_mb: number;
  java_overrides: Record<string, string>;
  msa_client_id: string | null;
  offline_username: string;
}

export interface AccountPublic {
  uuid: string;
  name: string;
  active: boolean;
}

export interface DeviceCode {
  user_code: string;
  device_code: string;
  verification_uri: string;
  interval: number;
  expires_in: number;
}

export type PollResult = { status: 'pending' } | { status: 'success'; account: AccountPublic };

export interface ProgressEvent {
  id: string;
  message: string;
  current: number;
  total: number;
  done: boolean;
}

export interface GameLogEvent {
  id: string;
  line: string;
}

export interface GameExitEvent {
  id: string;
  code: number;
  session_seconds: number;
}

/** Formata segundos como "3h 12min", "45min" ou "Nunca jogado". */
export function formatPlaytime(seconds: number): string {
  if (!seconds) return 'Nunca jogado';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return m > 0 ? `${h}h ${m}min` : `${h}h`;
  if (m > 0) return `${m}min`;
  return 'menos de 1min';
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    return Promise.reject(
      new Error('Recurso disponível apenas no app desktop (rode com "npm run tauri dev")'),
    );
  }
  return invoke<T>(cmd, args);
}

// Instâncias
export const listInstances = () => call<Instance[]>('list_instances');
export const createInstance = (name: string, gameVersion: string, loader: string, loaderVersion?: string) =>
  call<Instance>('create_instance', { name, gameVersion, loader, loaderVersion });
export const deleteInstance = (id: string) => call<void>('delete_instance', { id });
export const renameInstance = (id: string, name: string) => call<Instance>('rename_instance', { id, name });
export const duplicateInstance = (id: string) => call<Instance>('duplicate_instance', { id });
export const setInstanceIcon = (id: string, pngBase64: string) =>
  call<Instance>('set_instance_icon', { id, pngBase64 });
export const getInstanceIcon = (id: string) => call<string>('get_instance_icon', { id });
export const setInstancePinned = (id: string, pinned: boolean) =>
  call<Instance>('set_instance_pinned', { id, pinned });
export const setInstanceDetails = (id: string, notes: string | null, accentColor: string | null) =>
  call<Instance>('set_instance_details', { id, notes, accentColor });
export const openInstanceFolder = (id: string) => call<void>('open_instance_folder', { id });
export const listInstanceContent = (id: string) => call<ContentFile[]>('list_instance_content', { id });
export const removeInstanceContent = (id: string, folder: string, filename: string) =>
  call<void>('remove_instance_content', { id, folder, filename });

// Minecraft
export const getGameVersions = () => call<GameVersionEntry[]>('get_game_versions');
export const getLoaderVersions = (loader: string, gameVersion: string) =>
  call<string[]>('get_loader_versions', { loader, gameVersion });
export const prepareInstance = (id: string) => call<void>('prepare_instance', { id });
export const launchInstance = (id: string, username?: string) =>
  call<number>('launch_instance', { id, username });
export const killInstance = (id: string) => call<void>('kill_instance', { id });
export const getRunning = () => call<string[]>('get_running');

// Conteúdo
export const installContent = (instanceId: string, projectId: string, versionId?: string) =>
  call<string[]>('install_content', { instanceId, projectId, versionId });
export const installModpack = (projectId: string, versionId?: string) =>
  call<Instance>('install_modpack', { projectId, versionId });

// Atualização de mods e export
export interface ModUpdate {
  old_filename: string;
  new_filename: string;
  new_version: string;
  project_id: string;
}
export const checkModUpdates = (instanceId: string) =>
  call<ModUpdate[]>('check_mod_updates', { instanceId });
export const applyModUpdates = (instanceId: string) =>
  call<string[]>('apply_mod_updates', { instanceId });
export const exportModpack = (instanceId: string) =>
  call<string>('export_modpack', { instanceId });

// Configurações
export const getSettings = () => call<Settings>('get_settings');
export const setSettings = (settings: Settings) => call<Settings>('set_settings', { settings });

// Conta Microsoft
export const msaBegin = () => call<DeviceCode>('msa_begin');
export const msaPoll = (deviceCode: string) => call<PollResult>('msa_poll', { deviceCode });
export const getAccounts = () => call<AccountPublic[]>('get_accounts');
export const setActiveAccount = (uuid: string | null) => call<void>('set_active_account', { uuid });
export const removeAccount = (uuid: string) => call<void>('remove_account', { uuid });

// Skins
export interface SkinInfo {
  url: string | null;
  variant: string | null;
  capes: string[];
}
export const getSkin = () => call<SkinInfo>('get_skin');
export const uploadSkin = (pngBase64: string, variant: 'classic' | 'slim') =>
  call<void>('upload_skin', { pngBase64, variant });

// Galeria de skins
export interface SavedSkin {
  id: string;
  name: string;
  variant: 'classic' | 'slim';
  added: string;
  png_base64: string;
  favorite: boolean;
}
export const listSavedSkins = () => call<SavedSkin[]>('list_saved_skins');
export const addSavedSkin = (name: string, variant: 'classic' | 'slim', pngBase64: string) =>
  call<SavedSkin>('add_saved_skin', { name, variant, pngBase64 });
export const importSkinFromPlayer = (username: string) =>
  call<SavedSkin>('import_skin_from_player', { username });
export const importSkinFromUrl = (name: string, variant: 'classic' | 'slim', url: string) =>
  call<SavedSkin>('import_skin_from_url', { name, variant, url });
export const deleteSavedSkin = (id: string) => call<void>('delete_saved_skin', { id });
export const setFavoriteSkin = (id: string) => call<void>('set_favorite_skin', { id });
export const applySavedSkin = (id: string) => call<void>('apply_saved_skin', { id });

// Eventos
export function onProgress(handler: (e: ProgressEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(() => {});
  return listen<ProgressEvent>('progress', (event) => handler(event.payload));
}

export function onGameLog(handler: (e: GameLogEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(() => {});
  return listen<GameLogEvent>('game-log', (event) => handler(event.payload));
}

export function onGameExit(handler: (e: GameExitEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(() => {});
  return listen<GameExitEvent>('game-exit', (event) => handler(event.payload));
}
