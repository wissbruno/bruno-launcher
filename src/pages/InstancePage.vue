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
  type ContentFile,
} from '../api/backend';
import InstanceIcon from '../components/InstanceIcon.vue';

const route = useRoute();
const router = useRouter();
const store = useLauncherStore();

const id = computed(() => route.params.id as string);
const instance = computed(() => store.instances.find((i) => i.id === id.value) ?? null);
const isRunning = computed(() => store.running.has(id.value));
const logs = computed(() => store.logs.get(id.value) ?? []);

const tab = ref<'content' | 'logs'>('content');
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

function formatSize(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
</script>

<template>
  <div v-if="instance" class="instance-page">
    <button class="back" @click="router.push('/library')">← Biblioteca</button>

    <header class="card header">
      <InstanceIcon :instance="instance" :size="88" />
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
        <span v-if="isRunning" class="status running">● Em execução</span>
        <span v-else-if="!instance.installed" class="status pending">Jogo ainda não baixado</span>
      </div>
      <div class="head-actions">
        <button v-if="isRunning" class="stop" @click="stop">■ Parar</button>
        <button v-else class="btn-brand" @click="play">▶ Jogar</button>
        <button title="Abrir pasta" @click="openInstanceFolder(id)">📁</button>
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

    <nav class="tabs">
      <button :class="{ active: tab === 'content' }" @click="tab = 'content'; refreshContent()">
        Conteúdo
      </button>
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
