<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useLauncherStore } from '../stores/launcher';
import {
  launchInstance,
  killInstance,
  prepareInstance,
  deleteInstance,
  renameInstance,
  openInstanceFolder,
  listInstanceContent,
  removeInstanceContent,
  setInstanceIcon,
  setInstancePinned,
  setInstanceDetails,
  setInstanceJava,
  checkModUpdates,
  applyModUpdates,
  exportModpack,
  listWorlds,
  listScreenshots,
  getScreenshot,
  openInstanceSubfolder,
  formatPlaytime,
  formatDateShort,
  type ContentFile,
  type ModUpdate,
  type WorldInfo,
  type ScreenshotInfo,
} from '../api/backend';
import InstanceIcon from '../components/InstanceIcon.vue';

const route = useRoute();
const router = useRouter();
const store = useLauncherStore();

const id = computed(() => route.params.id as string);
const instance = computed(() => store.instances.find((i) => i.id === id.value) ?? null);
const isRunning = computed(() => store.running.has(id.value));
const logs = computed(() => store.logs.get(id.value) ?? []);

const tab = ref<'content' | 'worlds' | 'shots' | 'config' | 'logs'>('content');
const content = ref<ContentFile[]>([]);
const error = ref('');
const renaming = ref(false);
const newName = ref('');
const logsEl = ref<HTMLElement | null>(null);

const folderLabels: Record<string, string> = {
  mods: 'Mods',
  resourcepacks: 'Pacotes de textura',
  shaderpacks: 'Shaders',
  datapacks: 'Data packs',
};

const groupedContent = computed(() => {
  const groups: Record<string, ContentFile[]> = {};
  for (const f of content.value) {
    (groups[f.folder] ??= []).push(f);
  }
  return groups;
});

async function refreshContent() {
  try {
    content.value = await listInstanceContent(id.value);
  } catch {
    /* navegador */
  }
}

onMounted(async () => {
  await store.init();
  await store.refreshInstances();
  refreshContent();
});

watch(logs, async () => {
  if (tab.value === 'logs') {
    await nextTick();
    logsEl.value?.scrollTo({ top: logsEl.value.scrollHeight });
  }
});

async function play() {
  error.value = '';
  try {
    store.markRunning(id.value);
    tab.value = 'logs';
    await launchInstance(id.value, store.settings?.offline_username);
  } catch (e) {
    store.running.delete(id.value);
    error.value = String(e);
  }
}

async function stop() {
  await killInstance(id.value).catch((e) => (error.value = String(e)));
}

async function remove() {
  if (!confirm(`Excluir a instância "${instance.value?.name}"? Os mundos salvos serão perdidos.`)) return;
  await deleteInstance(id.value);
  await store.refreshInstances();
  router.push('/library');
}

async function saveRename() {
  if (newName.value.trim()) {
    await renameInstance(id.value, newName.value.trim());
    await store.refreshInstances();
  }
  renaming.value = false;
}

async function removeFile(file: ContentFile) {
  await removeInstanceContent(id.value, file.folder, file.filename);
  refreshContent();
}

async function onIconFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  try {
    const buf = await file.arrayBuffer();
    const base64 = btoa(String.fromCharCode(...new Uint8Array(buf)));
    await setInstanceIcon(id.value, base64);
    await store.refreshInstances();
  } catch (e) {
    error.value = String(e);
  } finally {
    (event.target as HTMLInputElement).value = '';
  }
}

async function clearIcon() {
  await setInstanceIcon(id.value, '');
  await store.refreshInstances();
}

async function togglePin() {
  if (!instance.value) return;
  await setInstancePinned(id.value, !instance.value.pinned);
  await store.refreshInstances();
}

// --- Atualizar mods ---
const updates = ref<ModUpdate[]>([]);
const checkingUpdates = ref(false);
const updateMsg = ref('');

async function checkUpdates() {
  checkingUpdates.value = true;
  updateMsg.value = '';
  error.value = '';
  try {
    updates.value = await checkModUpdates(id.value);
    updateMsg.value = updates.value.length
      ? `${updates.value.length} atualização(ões) disponível(is)`
      : 'Todos os mods estão atualizados ✓';
  } catch (e) {
    error.value = String(e);
  } finally {
    checkingUpdates.value = false;
  }
}

async function applyUpdates() {
  checkingUpdates.value = true;
  try {
    const done = await applyModUpdates(id.value);
    updateMsg.value = `${done.length} mod(s) atualizado(s)!`;
    updates.value = [];
    refreshContent();
  } catch (e) {
    error.value = String(e);
  } finally {
    checkingUpdates.value = false;
  }
}

// --- Notas e cor ---
const editingDetails = ref(false);
const notesDraft = ref('');
const colorDraft = ref('');
const accentColors = ['#1bd96a', '#4f9cff', '#c78aff', '#ffa347', '#ff496e', ''];

function openDetails() {
  notesDraft.value = instance.value?.notes ?? '';
  colorDraft.value = instance.value?.accent_color ?? '';
  editingDetails.value = true;
}

async function saveDetails() {
  await setInstanceDetails(id.value, notesDraft.value || null, colorDraft.value || null);
  await store.refreshInstances();
  editingDetails.value = false;
}

// --- Mundos e capturas ---
const worlds = ref<WorldInfo[]>([]);
const shots = ref<ScreenshotInfo[]>([]);
const shotImages = ref(new Map<string, string>());

async function loadWorlds() {
  tab.value = 'worlds';
  try {
    worlds.value = await listWorlds(id.value);
  } catch (e) {
    error.value = String(e);
  }
}

async function loadShots() {
  tab.value = 'shots';
  try {
    shots.value = await listScreenshots(id.value);
    // Carrega as 12 mais recentes
    for (const s of shots.value.slice(0, 12)) {
      if (!shotImages.value.has(s.filename)) {
        getScreenshot(id.value, s.filename)
          .then((b64) => shotImages.value.set(s.filename, b64))
          .catch(() => {});
      }
    }
  } catch (e) {
    error.value = String(e);
  }
}

// --- Configuração de Java por instância ---
const javaMemory = ref<number | null>(null);
const javaArgs = ref('');
const javaSaved = ref(false);

function openConfig() {
  tab.value = 'config';
  javaMemory.value = instance.value?.memory_mb ?? null;
  javaArgs.value = instance.value?.java_args ?? '';
}

async function saveJava() {
  await setInstanceJava(id.value, javaMemory.value || null, javaArgs.value || null);
  await store.refreshInstances();
  javaSaved.value = true;
  setTimeout(() => (javaSaved.value = false), 2000);
}

// --- Exportar modpack ---
const exporting = ref(false);

async function doExport() {
  exporting.value = true;
  error.value = '';
  try {
    const path = await exportModpack(id.value);
    updateMsg.value = `Modpack exportado em: ${path}`;
  } catch (e) {
    error.value = String(e);
  } finally {
    exporting.value = false;
  }
}

function formatSize(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
</script>

<template>
  <div v-if="instance" class="instance-page">
    <button class="back" @click="router.push('/library')">← Biblioteca</button>

    <header
      class="card header"
      :style="instance.accent_color ? { borderLeft: `4px solid ${instance.accent_color}` } : {}"
    >
      <label class="icon-edit" title="Trocar ícone da instância">
        <InstanceIcon :instance="instance" :size="88" />
        <span class="icon-overlay">✎</span>
        <input type="file" accept="image/png,image/jpeg" hidden @change="onIconFile" />
      </label>
      <div class="head-info">
        <div v-if="renaming" class="rename-row">
          <input v-model="newName" @keyup.enter="saveRename" />
          <button @click="saveRename">Salvar</button>
        </div>
        <h1 v-else @dblclick="renaming = true; newName = instance.name">{{ instance.name }}</h1>
        <span class="meta">
          {{ instance.loader }} · Minecraft {{ instance.game_version }}
          <template v-if="instance.loader_version"> · loader {{ instance.loader_version }}</template>
        </span>
        <span class="meta">🕑 {{ formatPlaytime(instance.playtime_seconds) }} de jogatina</span>
        <span v-if="isRunning" class="status running">● Em execução</span>
        <span v-else-if="!instance.installed" class="status pending">Jogo ainda não baixado</span>
      </div>
      <div class="head-actions">
        <button v-if="isRunning" class="stop" @click="stop">■ Parar</button>
        <button v-else class="btn-brand" @click="play">▶ Jogar</button>
        <button
          class="pin"
          :class="{ active: instance.pinned }"
          :title="instance.pinned ? 'Desafixar' : 'Fixar no topo'"
          @click="togglePin"
        >
          {{ instance.pinned ? '★' : '☆' }}
        </button>
        <button title="Abrir pasta" @click="openInstanceFolder(id)">📁</button>
        <button v-if="instance.custom_icon" title="Remover ícone" @click="clearIcon">🖼</button>
        <button
          v-if="!instance.installed"
          title="Baixar arquivos do jogo"
          @click="prepareInstance(id).catch((e) => (error = String(e)))"
        >
          ⬇
        </button>
        <button class="danger" title="Excluir instância" @click="remove">🗑</button>
      </div>
    </header>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-if="updateMsg" class="ok">{{ updateMsg }}</p>

    <section v-if="instance.notes || editingDetails" class="card notes-card">
      <template v-if="editingDetails">
        <textarea v-model="notesDraft" placeholder="Anotações sobre esta instância..." rows="3"></textarea>
        <div class="color-row">
          <span>Cor:</span>
          <button
            v-for="c in accentColors"
            :key="c || 'none'"
            class="swatch"
            :class="{ sel: colorDraft === c }"
            :style="{ background: c || 'transparent', borderColor: c || 'var(--color-divider)' }"
            :title="c || 'Sem cor'"
            @click="colorDraft = c"
          >
            {{ c ? '' : '∅' }}
          </button>
        </div>
        <div class="notes-actions">
          <button @click="editingDetails = false">Cancelar</button>
          <button class="btn-brand" @click="saveDetails">Salvar</button>
        </div>
      </template>
      <template v-else>
        <p class="notes-text">{{ instance.notes }}</p>
        <button class="small" @click="openDetails">Editar</button>
      </template>
    </section>

    <nav class="tabs">
      <button :class="{ active: tab === 'content' }" @click="tab = 'content'; refreshContent()">
        Conteúdo
      </button>
      <button :class="{ active: tab === 'worlds' }" @click="loadWorlds">Mundos</button>
      <button :class="{ active: tab === 'shots' }" @click="loadShots">Capturas</button>
      <button :class="{ active: tab === 'config' }" @click="openConfig">Configuração</button>
      <button :class="{ active: tab === 'logs' }" @click="tab = 'logs'">Logs do jogo</button>
    </nav>

    <section v-if="tab === 'content'">
      <div class="content-header">
        <button
          class="btn-brand"
          @click="router.push({ path: '/browse/mod', query: { instance: id } })"
        >
          + Adicionar conteúdo
        </button>
        <button :disabled="checkingUpdates" @click="checkUpdates">
          {{ checkingUpdates ? 'Verificando...' : '🔄 Verificar atualizações' }}
        </button>
        <button v-if="updates.length" class="btn-brand" :disabled="checkingUpdates" @click="applyUpdates">
          ⬆ Atualizar {{ updates.length }} mod(s)
        </button>
        <button :disabled="exporting" @click="doExport">
          {{ exporting ? 'Exportando...' : '📦 Exportar .mrpack' }}
        </button>
        <button v-if="!instance.notes && !editingDetails" @click="openDetails">📝 Notas/cor</button>
      </div>

      <div v-if="updates.length" class="updates card">
        <div v-for="up in updates" :key="up.old_filename" class="update-row">
          <span>{{ up.old_filename }}</span>
          <span class="arrow">→</span>
          <span class="new">{{ up.new_filename }}</span>
        </div>
      </div>
      <template v-if="content.length">
        <div v-for="(files, folder) in groupedContent" :key="folder" class="group">
          <h3>{{ folderLabels[folder] ?? folder }} ({{ files.length }})</h3>
          <article v-for="file in files" :key="file.filename" class="card file-row">
            <span class="filename">{{ file.filename }}</span>
            <span class="size">{{ formatSize(file.size) }}</span>
            <button class="danger small" title="Remover" @click="removeFile(file)">✕</button>
          </article>
        </div>
      </template>
      <p v-else class="empty">
        Nenhum conteúdo instalado ainda — use "Adicionar conteúdo" para buscar mods compatíveis.
      </p>
    </section>

    <section v-else-if="tab === 'worlds'">
      <div class="content-header">
        <button @click="openInstanceSubfolder(id, 'saves')">📁 Abrir pasta de mundos</button>
      </div>
      <div v-if="worlds.length" class="worlds-grid">
        <article v-for="w in worlds" :key="w.folder" class="card world-card">
          <img
            v-if="w.icon_base64"
            :src="`data:image/png;base64,${w.icon_base64}`"
            class="world-icon"
            alt=""
          />
          <div v-else class="world-icon placeholder">🌍</div>
          <div class="world-info">
            <strong>{{ w.folder }}</strong>
            <span v-if="w.last_modified" class="date">
              Jogado em {{ formatDateShort(w.last_modified) }}
            </span>
          </div>
        </article>
      </div>
      <p v-else class="empty">Nenhum mundo ainda — os saves aparecem aqui depois de jogar.</p>
    </section>

    <section v-else-if="tab === 'shots'">
      <div class="content-header">
        <button @click="openInstanceSubfolder(id, 'screenshots')">📁 Abrir pasta de capturas</button>
      </div>
      <div v-if="shots.length" class="shots-grid">
        <figure v-for="s in shots.slice(0, 12)" :key="s.filename" class="card shot">
          <img
            v-if="shotImages.get(s.filename)"
            :src="`data:image/png;base64,${shotImages.get(s.filename)}`"
            :alt="s.filename"
            loading="lazy"
          />
          <div v-else class="shot-loading">Carregando...</div>
          <figcaption>{{ s.filename }}</figcaption>
        </figure>
      </div>
      <p v-else class="empty">
        Nenhuma captura — aperte F2 dentro do jogo e elas aparecem aqui.
      </p>
    </section>

    <section v-else-if="tab === 'config'" class="card config">
      <h3>Java desta instância</h3>
      <label class="field">
        Memória máxima (MB) — vazio usa o padrão global
        <input
          v-model.number="javaMemory"
          type="number"
          min="512"
          step="512"
          placeholder="padrão global"
          style="max-width: 200px"
        />
      </label>
      <label class="field">
        Argumentos extras da JVM
        <input
          v-model="javaArgs"
          placeholder="-XX:+UseG1GC ..."
          style="max-width: 420px"
        />
      </label>
      <button class="btn-brand" style="align-self: flex-start" @click="saveJava">
        {{ javaSaved ? 'Salvo ✓' : 'Salvar' }}
      </button>
    </section>

    <section v-else ref="logsEl" class="card logs">
      <pre v-if="logs.length">{{ logs.join('\n') }}</pre>
      <p v-else class="empty">Os logs aparecem aqui quando o jogo é iniciado.</p>
    </section>
  </div>

  <div v-else class="instance-page">
    <button class="back" @click="router.push('/library')">← Biblioteca</button>
    <p class="empty">Instância não encontrada.</p>
  </div>
</template>

<style scoped>
.back {
  margin-bottom: 1rem;
}

.header {
  display: flex;
  gap: 1rem;
  align-items: center;
}

.icon-edit {
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  display: block;
}

.icon-overlay {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  border-radius: var(--radius-md);
  opacity: 0;
  transition: opacity 0.15s;
  font-size: 24px;
}

.icon-edit:hover .icon-overlay {
  opacity: 1;
}

.pin.active {
  color: var(--color-orange);
}

.notes-card {
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.notes-text {
  margin: 0;
  color: var(--color-base);
  white-space: pre-wrap;
}

.notes-card textarea {
  width: 100%;
  resize: vertical;
  font-family: inherit;
  background: var(--color-button-bg);
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-contrast);
  padding: 0.6rem;
  outline: none;
}

.color-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 13px;
  color: var(--color-secondary);
}

.swatch {
  width: 26px;
  height: 26px;
  padding: 0;
  border-radius: 50%;
  border: 2px solid;
  color: var(--color-secondary);
}

.swatch.sel {
  box-shadow: 0 0 0 2px var(--color-contrast);
}

.notes-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}

.updates {
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.update-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 13px;
}

.update-row .arrow {
  color: var(--color-secondary);
}

.update-row .new {
  color: var(--color-brand);
}

.content-header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.ok {
  color: var(--color-brand);
  word-break: break-all;
}

.worlds-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 0.75rem;
}

.world-card {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.75rem;
}

.world-icon {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-md);
  object-fit: cover;
  image-rendering: pixelated;
}

.world-icon.placeholder {
  display: grid;
  place-items: center;
  background: var(--color-button-bg);
  font-size: 24px;
}

.world-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.world-info strong {
  color: var(--color-contrast);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.world-info .date {
  font-size: 12px;
  color: var(--color-secondary);
}

.shots-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 0.75rem;
}

.shot {
  margin: 0;
  padding: 0.5rem;
}

.shot img {
  width: 100%;
  border-radius: var(--radius-md);
}

.shot-loading {
  height: 150px;
  display: grid;
  place-items: center;
  color: var(--color-secondary);
  font-size: 13px;
}

.shot figcaption {
  padding: 0.4rem 0.25rem 0;
  font-size: 12px;
  color: var(--color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  max-width: 560px;
}

.config .field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 13px;
  color: var(--color-secondary);
}

.head-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.meta {
  color: var(--color-secondary);
  font-size: 13px;
  text-transform: capitalize;
}

.status.running {
  color: var(--color-brand);
  font-size: 13px;
}

.status.pending {
  color: var(--color-orange);
  font-size: 13px;
}

.head-actions {
  display: flex;
  gap: 0.5rem;
}

.stop {
  color: var(--color-red);
}

.danger {
  color: var(--color-red);
}

.rename-row {
  display: flex;
  gap: 0.5rem;
}

.tabs {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}

.tabs button {
  background: transparent;
  color: var(--color-secondary);
}

.tabs button.active {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.content-header {
  margin-bottom: 1rem;
}

.group {
  margin-bottom: 1.25rem;
}

.group h3 {
  font-size: 14px;
  margin-bottom: 0.5rem;
  color: var(--color-secondary);
}

.file-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.85rem;
  margin-bottom: 0.35rem;
}

.filename {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13.5px;
  color: var(--color-contrast);
}

.size {
  font-size: 12px;
  color: var(--color-secondary);
}

.small {
  padding: 0.25rem 0.55rem;
}

.logs {
  max-height: calc(100vh - 320px);
  overflow-y: auto;
  user-select: text;
}

.logs pre {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}

.empty {
  color: var(--color-secondary);
}

.error {
  color: var(--color-red);
}
</style>
