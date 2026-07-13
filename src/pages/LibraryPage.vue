<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useLauncherStore } from '../stores/launcher';
import {
  launchInstance,
  killInstance,
  prepareInstance,
  duplicateInstance,
  formatPlaytime,
} from '../api/backend';
import CreateInstanceModal from '../components/CreateInstanceModal.vue';
import InstanceIcon from '../components/InstanceIcon.vue';
import type { Instance } from '../api/backend';

const store = useLauncherStore();
const router = useRouter();
const showCreate = ref(false);
const error = ref('');
const sortBy = ref<'recent' | 'played' | 'name'>('recent');

const sortedInstances = computed(() => {
  const list = [...store.instances];
  if (sortBy.value === 'played') list.sort((a, b) => b.playtime_seconds - a.playtime_seconds);
  else if (sortBy.value === 'name') list.sort((a, b) => a.name.localeCompare(b.name));
  // 'recent' já vem ordenado por criação do backend
  // Fixadas sempre no topo, preservando a ordem escolhida
  return list.sort((a, b) => Number(b.pinned) - Number(a.pinned));
});

const totalPlaytime = computed(() =>
  store.instances.reduce((sum, i) => sum + i.playtime_seconds, 0),
);

onMounted(() => {
  store.init();
  store.refreshInstances();
});

async function play(instance: Instance) {
  error.value = '';
  try {
    store.markRunning(instance.id);
    await launchInstance(instance.id, store.settings?.offline_username);
  } catch (e) {
    store.running.delete(instance.id);
    error.value = String(e);
  }
}

async function stop(instance: Instance) {
  try {
    await killInstance(instance.id);
  } catch (e) {
    error.value = String(e);
  }
}

function retryInstall(instance: Instance) {
  prepareInstance(instance.id).catch((e) => (error.value = String(e)));
}

async function duplicate(instance: Instance) {
  try {
    await duplicateInstance(instance.id);
    await store.refreshInstances();
  } catch (e) {
    error.value = String(e);
  }
}

const fmt = formatPlaytime;

const loaderLabel: Record<string, string> = {
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
};
</script>

<template>
  <div class="library">
    <div class="header-row">
      <div>
        <h1>Biblioteca</h1>
        <p class="subtitle">Suas instalações do Minecraft.</p>
      </div>
      <button class="btn-brand" :disabled="!store.isTauri" @click="showCreate = true">
        + Nova instância
      </button>
    </div>

    <p v-if="!store.isTauri" class="warn card">
      Você está no navegador — instâncias funcionam apenas no app desktop
      (<code>npm run tauri dev</code>).
    </p>

    <div v-if="store.instances.length" class="stats-bar">
      <div class="stat">
        <strong>{{ store.instances.length }}</strong>
        <span>{{ store.instances.length === 1 ? 'instância' : 'instâncias' }}</span>
      </div>
      <div class="stat">
        <strong>{{ fmt(totalPlaytime) }}</strong>
        <span>de jogatina total</span>
      </div>
      <div class="spacer"></div>
      <label class="sort">
        Ordenar:
        <select v-model="sortBy">
          <option value="recent">Mais recentes</option>
          <option value="played">Mais jogadas</option>
          <option value="name">Nome</option>
        </select>
      </label>
    </div>

    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="store.instances.length" class="grid">
      <article
        v-for="instance in sortedInstances"
        :key="instance.id"
        class="card instance"
        @click="router.push(`/instance/${instance.id}`)"
      >
        <InstanceIcon :instance="instance" :size="64" />
        <div class="info">
          <h3>
            <span v-if="instance.pinned" class="pin-badge" title="Fixada">★</span>
            {{ instance.name }}
          </h3>
          <span class="meta">
            {{ loaderLabel[instance.loader] ?? instance.loader }} {{ instance.game_version }}
          </span>
          <span v-if="store.running.has(instance.id)" class="status running">● Em execução</span>
          <span v-else-if="!instance.installed" class="status pending">Aguardando download</span>
          <span v-else class="playtime">🕑 {{ fmt(instance.playtime_seconds) }}</span>
        </div>
        <div class="actions" @click.stop>
          <button
            v-if="store.running.has(instance.id)"
            class="stop"
            title="Parar o jogo"
            @click="stop(instance)"
          >
            ■
          </button>
          <button
            v-else-if="instance.installed"
            class="btn-brand play"
            title="Jogar"
            @click="play(instance)"
          >
            ▶
          </button>
          <button
            v-else
            class="play"
            title="Baixar o jogo"
            @click="retryInstall(instance)"
          >
            ⬇
          </button>
          <button class="dup" title="Duplicar instância" @click="duplicate(instance)">⧉</button>
        </div>
      </article>
    </div>

    <div v-else-if="store.isTauri" class="card empty-state">
      <h3>Nenhuma instância ainda</h3>
      <p>Crie uma instalação do Minecraft ou instale um modpack pela aba Descobrir.</p>
      <button class="btn-brand" @click="showCreate = true">+ Criar instância</button>
    </div>

    <CreateInstanceModal v-if="showCreate" @close="showCreate = false" />
  </div>
</template>

<style scoped>
.header-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 1.5rem;
}

.subtitle {
  color: var(--color-secondary);
  margin: 0.35rem 0 0;
}

.stats-bar {
  display: flex;
  align-items: center;
  gap: 2rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  padding: 0.85rem 1.25rem;
  margin-bottom: 1rem;
}

.stat {
  display: flex;
  flex-direction: column;
}

.stat strong {
  color: var(--color-contrast);
  font-size: 18px;
}

.stat span {
  font-size: 12px;
  color: var(--color-secondary);
}

.spacer {
  flex: 1;
}

.sort {
  font-size: 13px;
  color: var(--color-secondary);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.playtime {
  font-size: 12px;
  color: var(--color-secondary);
}

.pin-badge {
  color: var(--color-orange);
  font-size: 12px;
}

.dup {
  width: 42px;
  height: 42px;
  padding: 0;
  font-size: 15px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 0.75rem;
}

.instance {
  display: flex;
  gap: 0.85rem;
  align-items: center;
  cursor: pointer;
  transition: filter 0.15s;
}

.instance:hover {
  filter: brightness(1.15);
}

.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.info h3 {
  font-size: 15px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta {
  font-size: 12.5px;
  color: var(--color-secondary);
}

.status {
  font-size: 12px;
}

.status.running {
  color: var(--color-brand);
}

.status.pending {
  color: var(--color-orange);
}

.actions {
  display: flex;
  gap: 0.4rem;
}

.play,
.stop {
  width: 42px;
  height: 42px;
  padding: 0;
  font-size: 16px;
  display: grid;
  place-items: center;
}

.stop {
  color: var(--color-red);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 3rem 2rem;
  text-align: center;
  max-width: 480px;
  margin: 3rem auto;
}

.empty-state p {
  color: var(--color-secondary);
  font-size: 13.5px;
  margin: 0;
}

.warn {
  color: var(--color-orange);
  margin-bottom: 1rem;
}

.error {
  color: var(--color-red);
}
</style>
