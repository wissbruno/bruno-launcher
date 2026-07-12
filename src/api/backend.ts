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
  loader: 'vanilla' | 'fabric' | 'quilt';
  loader_version: string | null;
  created: string;
  last_played: string | null;
  icon_url: string | null;
  modpack: string | null;
  installed: boolean;
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

// Configurações
export const getSettings = () => call<Settings>('get_settings');
export const setSettings = (settings: Settings) => call<Settings>('set_settings', { settings });

// Conta Microsoft
export const msaBegin = () => call<DeviceCode>('msa_begin');
export const msaPoll = (deviceCode: string) => call<PollResult>('msa_poll', { deviceCode });
export const getAccounts = () => call<AccountPublic[]>('get_accounts');
export const setActiveAccount = (uuid: string | null) => call<void>('set_active_account', { uuid });
export const removeAccount = (uuid: string) => call<void>('remove_account', { uuid });

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
