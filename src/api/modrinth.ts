/**
 * Cliente da API pública do Modrinth (labrinth).
 * Documentação: https://docs.modrinth.com/api
 *
 * Durante o desenvolvimento no navegador usamos fetch direto (a API permite CORS).
 * Quando o backend Rust estiver pronto, downloads e operações pesadas migram
 * para comandos Tauri, mantendo esta camada como fonte de dados da interface.
 */

const API_BASE = 'https://api.modrinth.com/v2';

export type ProjectType = 'mod' | 'modpack' | 'resourcepack' | 'shader' | 'datapack';

export interface SearchHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  categories: string[];
  display_categories: string[];
  project_type: ProjectType;
  downloads: number;
  follows: number;
  icon_url: string | null;
  author: string;
  versions: string[];
  latest_version: string;
  date_modified: string;
  client_side: string;
  server_side: string;
  color: number | null;
}

export interface SearchResponse {
  hits: SearchHit[];
  offset: number;
  limit: number;
  total_hits: number;
}

export interface Project {
  id: string;
  slug: string;
  title: string;
  description: string;
  body: string; // markdown
  project_type: ProjectType;
  downloads: number;
  followers: number;
  icon_url: string | null;
  categories: string[];
  game_versions: string[];
  loaders: string[];
  published: string;
  updated: string;
  source_url: string | null;
  issues_url: string | null;
  wiki_url: string | null;
  discord_url: string | null;
  gallery: { url: string; featured: boolean; title: string | null; description: string | null }[];
  team: string;
}

export interface VersionFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: { sha1: string; sha512: string };
}

export interface Version {
  id: string;
  project_id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  version_type: 'release' | 'beta' | 'alpha';
  downloads: number;
  date_published: string;
  files: VersionFile[];
  dependencies: { project_id: string | null; version_id: string | null; dependency_type: string }[];
}

export interface Category {
  icon: string;
  name: string;
  project_type: string;
  header: string;
}

export interface Loader {
  icon: string;
  name: string;
  supported_project_types: string[];
}

export interface GameVersion {
  version: string;
  version_type: 'release' | 'snapshot' | 'alpha' | 'beta';
  date: string;
  major: boolean;
}

export type SortIndex = 'relevance' | 'downloads' | 'follows' | 'newest' | 'updated';

export interface SearchParams {
  query?: string;
  projectType?: ProjectType;
  categories?: string[];
  loaders?: string[];
  gameVersion?: string;
  index?: SortIndex;
  offset?: number;
  limit?: number;
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) {
    throw new Error(`Modrinth API ${res.status}: ${await res.text()}`);
  }
  return res.json() as Promise<T>;
}

export async function searchProjects(params: SearchParams): Promise<SearchResponse> {
  const facets: string[][] = [];
  if (params.projectType) facets.push([`project_type:${params.projectType}`]);
  if (params.categories?.length) facets.push(params.categories.map((c) => `categories:${c}`));
  if (params.loaders?.length) facets.push(params.loaders.map((l) => `categories:${l}`));
  if (params.gameVersion) facets.push([`versions:${params.gameVersion}`]);

  const qs = new URLSearchParams();
  if (params.query) qs.set('query', params.query);
  if (facets.length) qs.set('facets', JSON.stringify(facets));
  qs.set('index', params.index ?? 'relevance');
  qs.set('offset', String(params.offset ?? 0));
  qs.set('limit', String(params.limit ?? 20));

  return get<SearchResponse>(`/search?${qs}`);
}

export function getProject(idOrSlug: string): Promise<Project> {
  return get<Project>(`/project/${idOrSlug}`);
}

export function getProjectVersions(idOrSlug: string): Promise<Version[]> {
  return get<Version[]>(`/project/${idOrSlug}/version`);
}

export function getCategories(): Promise<Category[]> {
  return get<Category[]>('/tag/category');
}

export function getLoaders(): Promise<Loader[]> {
  return get<Loader[]>('/tag/loader');
}

export function getGameVersions(): Promise<GameVersion[]> {
  return get<GameVersion[]>('/tag/game_version');
}

const compact = new Intl.NumberFormat('pt-BR', { notation: 'compact', maximumFractionDigits: 1 });

export function formatCount(n: number): string {
  return compact.format(n);
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('pt-BR', { day: 'numeric', month: 'short', year: 'numeric' });
}
