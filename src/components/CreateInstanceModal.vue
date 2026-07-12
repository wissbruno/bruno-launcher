<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue';
import {
  createInstance,
  prepareInstance,
  getGameVersions,
  getLoaderVersions,
  type GameVersionEntry,
} from '../api/backend';
import { useLauncherStore } from '../stores/launcher';

const emit = defineEmits<{ close: [] }>();
const store = useLauncherStore();

const name = ref('');
const loader = ref<'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'>('vanilla');

const loaderLabels = {
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
} as const;
const gameVersion = ref('');
const showSnapshots = ref(false);
const versions = ref<GameVersionEntry[]>([]);
const loaderVersions = ref<string[]>([]);
const loaderVersion = ref('');
const busy = ref(false);
const error = ref('');

const visibleVersions = computed(() =>
  versions.value.filter((v) => showSnapshots.value || v.version_type === 'release'),
);

onMounted(async () => {
  try {
    versions.value = await getGameVersions();
    const firstRelease = versions.value.find((v) => v.version_type === 'release');
    if (firstRelease) gameVersion.value = firstRelease.id;
  } catch (e) {
    error.value = String(e);
  }
});

watch([loader, gameVersion], async () => {
  loaderVersions.value = [];
  loaderVersion.value = '';
  error.value = '';
  if (loader.value === 'vanilla' || !gameVersion.value) return;
  try {
    loaderVersions.value = await getLoaderVersions(loader.value, gameVersion.value);
    loaderVersion.value = loaderVersions.value[0] ?? '';
  } catch (e) {
    error.value = String(e);
  }
});

async function create() {
  if (!name.value.trim() || !gameVersion.value) return;
  busy.value = true;
  error.value = '';
  try {
    const instance = await createInstance(
      name.value.trim(),
      gameVersion.value,
      loader.value,
      loader.value === 'vanilla' ? undefined : loaderVersion.value,
    );
    await store.refreshInstances();
    // Baixa o jogo em segundo plano; o painel de progresso mostra o andamento
    prepareInstance(instance.id).catch(() => {});
    emit('close');
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="modal card">
      <h2>Nova instância</h2>

      <label class="field">
        Nome
        <input v-model="name" placeholder="Minha instância" autofocus @keyup.enter="create" />
      </label>

      <label class="field">
        Mod loader
        <div class="loader-row">
          <button
            v-for="(label, l) in loaderLabels"
            :key="l"
            :class="{ selected: loader === l }"
            @click="loader = l"
          >
            {{ label }}
          </button>
        </div>
      </label>

      <label class="field">
        Versão do Minecraft
        <select v-model="gameVersion">
          <option v-for="v in visibleVersions" :key="v.id" :value="v.id">
            {{ v.id }}{{ v.version_type !== 'release' ? ` (${v.version_type})` : '' }}
          </option>
        </select>
        <label class="check">
          <input v-model="showSnapshots" type="checkbox" />
          Mostrar snapshots
        </label>
      </label>

      <label v-if="loader !== 'vanilla'" class="field">
        Versão do {{ loader }}
        <select v-model="loaderVersion">
          <option v-for="v in loaderVersions" :key="v" :value="v">{{ v }}</option>
        </select>
      </label>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button @click="emit('close')">Cancelar</button>
        <button class="btn-brand" :disabled="busy || !name.trim()" @click="create">
          {{ busy ? 'Criando...' : 'Criar e baixar' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: grid;
  place-items: center;
  z-index: 100;
}

.modal {
  width: min(440px, 90vw);
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  padding: 1.5rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  font-size: 13px;
  color: var(--color-secondary);
}

.loader-row {
  display: flex;
  gap: 0.5rem;
}

.loader-row button.selected {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.check {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 12px;
}

.check input {
  accent-color: var(--color-brand);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.error {
  color: var(--color-red);
  font-size: 13px;
  margin: 0;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
