<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useLauncherStore } from '../stores/launcher';
import { installContent, installModpack, prepareInstance } from '../api/backend';
import InstanceIcon from '../components/InstanceIcon.vue';
import type { Project, Version } from '../api/modrinth';

const props = defineProps<{
  project: Project;
  version?: Version | null;
}>();
const emit = defineEmits<{ close: [] }>();

const store = useLauncherStore();
const router = useRouter();
const busy = ref(false);
const error = ref('');

const isModpack = computed(() => props.project.project_type === 'modpack');

/** Instância compatível: versão do jogo bate; para mods, o loader também. */
function isCompatible(instanceGameVersion: string, instanceLoader: string): boolean {
  const gv = props.version?.game_versions ?? props.project.game_versions;
  if (!gv.includes(instanceGameVersion)) return false;
  if (props.project.project_type !== 'mod') return true;
  const loaders = props.version?.loaders ?? props.project.loaders;
  return loaders.includes(instanceLoader);
}

const sorted = computed(() =>
  [...store.instances].sort((a, b) => {
    const ca = isCompatible(a.game_version, a.loader) ? 0 : 1;
    const cb = isCompatible(b.game_version, b.loader) ? 0 : 1;
    return ca - cb;
  }),
);

onMounted(() => {
  store.init();
  store.refreshInstances();
});

async function installTo(instanceId: string) {
  busy.value = true;
  error.value = '';
  try {
    await installContent(instanceId, props.project.id, props.version?.id);
    emit('close');
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function installAsInstance() {
  busy.value = true;
  error.value = '';
  try {
    const instance = await installModpack(props.project.id, props.version?.id);
    await store.refreshInstances();
    // baixa o jogo/loader em segundo plano
    prepareInstance(instance.id).catch(() => {});
    emit('close');
    router.push(`/instance/${instance.id}`);
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
      <h2>Instalar {{ project.title }}</h2>
      <p v-if="version" class="hint">Versão: {{ version.name }}</p>

      <p v-if="!store.isTauri" class="warn">
        Instalação disponível apenas no app desktop.
      </p>

      <template v-else-if="isModpack">
        <p class="hint">Modpacks são instalados como uma nova instância.</p>
        <button class="btn-brand" :disabled="busy" @click="installAsInstance">
          {{ busy ? 'Instalando...' : '+ Criar instância com este modpack' }}
        </button>
      </template>

      <template v-else>
        <p class="hint">Escolha a instância:</p>
        <div class="list">
          <button
            v-for="instance in sorted"
            :key="instance.id"
            class="instance-row"
            :class="{ incompatible: !isCompatible(instance.game_version, instance.loader) }"
            :disabled="busy"
            @click="installTo(instance.id)"
          >
            <InstanceIcon :instance="instance" :size="36" />
            <span class="name">{{ instance.name }}</span>
            <span class="meta">{{ instance.loader }} {{ instance.game_version }}</span>
          </button>
        </div>
        <p v-if="!store.instances.length" class="hint">
          Você ainda não tem instâncias — crie uma na Biblioteca.
        </p>
      </template>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button @click="emit('close')">Fechar</button>
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
  width: min(460px, 90vw);
  max-height: 80vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1.5rem;
}

.hint {
  color: var(--color-secondary);
  font-size: 13px;
  margin: 0;
}

.warn {
  color: var(--color-orange);
}

.list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.instance-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  text-align: left;
  padding: 0.5rem 0.75rem;
}

.instance-row .name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-row .meta {
  font-size: 12px;
  color: var(--color-secondary);
  text-transform: capitalize;
}

.instance-row.incompatible {
  opacity: 0.45;
}

.actions {
  display: flex;
  justify-content: flex-end;
}

.error {
  color: var(--color-red);
  font-size: 13px;
  margin: 0;
}
</style>
