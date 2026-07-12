<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useLauncherStore } from '../stores/launcher';
import {
  msaBegin,
  msaPoll,
  setActiveAccount,
  removeAccount,
  type DeviceCode,
} from '../api/backend';

const store = useLauncherStore();

const saved = ref(false);
const error = ref('');

// --- Login Microsoft ---
const device = ref<DeviceCode | null>(null);
const loggingIn = ref(false);
const loginError = ref('');
let pollTimer: ReturnType<typeof setInterval> | null = null;

const local = computed(() => store.settings);

onMounted(() => store.init());

async function save() {
  if (!local.value) return;
  error.value = '';
  try {
    await store.saveSettings({ ...local.value });
    saved.value = true;
    setTimeout(() => (saved.value = false), 2000);
  } catch (e) {
    error.value = String(e);
  }
}

async function startLogin() {
  loginError.value = '';
  loggingIn.value = true;
  try {
    device.value = await msaBegin();
    const interval = Math.max(device.value.interval * 1000, 3000);
    pollTimer = setInterval(async () => {
      if (!device.value) return;
      try {
        const result = await msaPoll(device.value.device_code);
        if (result.status === 'success') {
          stopLogin();
          await store.refreshAccounts();
        }
      } catch (e) {
        stopLogin();
        loginError.value = String(e);
      }
    }, interval);
  } catch (e) {
    loginError.value = String(e);
    loggingIn.value = false;
  }
}

function stopLogin() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = null;
  device.value = null;
  loggingIn.value = false;
}

async function activate(uuid: string) {
  await setActiveAccount(uuid);
  await store.refreshAccounts();
}

async function logout(uuid: string) {
  await removeAccount(uuid);
  await store.refreshAccounts();
}
</script>

<template>
  <div class="settings">
    <h1>Configurações</h1>

    <section class="card group">
      <h3>Conta Minecraft</h3>

      <template v-if="store.accounts.length">
        <div v-for="account in store.accounts" :key="account.uuid" class="account-row">
          <img
            :src="`https://mc-heads.net/avatar/${account.uuid}/32`"
            class="head"
            alt=""
          />
          <span class="name">{{ account.name }}</span>
          <span v-if="account.active" class="active-badge">ativa</span>
          <button v-else class="small" @click="activate(account.uuid)">Usar</button>
          <button class="small danger" @click="logout(account.uuid)">Sair</button>
        </div>
      </template>
      <p v-else class="hint">
        Nenhuma conta conectada — o jogo roda em modo offline. Para entrar com sua conta
        Microsoft, você precisa de um Client ID do Azure aprovado pela Mojang
        (<a
          href="https://help.minecraft.net/hc/en-us/articles/16254801392141"
          target="_blank"
        >como conseguir</a>) configurado abaixo.
      </p>

      <div v-if="device" class="device-code card">
        <p>
          1. Abra
          <a :href="device.verification_uri" target="_blank">{{ device.verification_uri }}</a>
        </p>
        <p>2. Digite o código:</p>
        <strong class="code">{{ device.user_code }}</strong>
        <p class="hint">Aguardando você concluir o login no navegador...</p>
        <button class="small" @click="stopLogin">Cancelar</button>
      </div>
      <button v-else :disabled="loggingIn || !store.isTauri" @click="startLogin">
        Entrar com Microsoft
      </button>
      <p v-if="loginError" class="error">{{ loginError }}</p>
    </section>

    <section v-if="local" class="card group">
      <h3>Jogo</h3>

      <label class="field">
        Nome de jogador (modo offline)
        <input v-model="local.offline_username" style="max-width: 240px" />
      </label>

      <label class="field">
        Memória máxima: {{ (local.memory_mb / 1024).toFixed(1) }} GB
        <input
          v-model.number="local.memory_mb"
          type="range"
          min="1024"
          max="16384"
          step="512"
          style="max-width: 320px"
        />
      </label>

      <label class="field">
        Client ID do Azure (login Microsoft)
        <input
          :value="local.msa_client_id ?? ''"
          placeholder="00000000-0000-0000-0000-000000000000"
          style="max-width: 360px"
          @input="local.msa_client_id = ($event.target as HTMLInputElement).value || null"
        />
      </label>

      <button class="btn-brand" @click="save">{{ saved ? 'Salvo ✓' : 'Salvar' }}</button>
      <p v-if="error" class="error">{{ error }}</p>
    </section>

    <section class="card group">
      <h3>Sobre</h3>
      <p class="hint">
        Réplica do Modrinth App feita para aprendizado — Tauri 2, Rust, Vue 3. Dados da
        <a href="https://docs.modrinth.com" target="_blank">API pública do Modrinth</a>; jogo
        baixado dos servidores oficiais da Mojang; Java por
        <a href="https://adoptium.net" target="_blank">Eclipse Adoptium</a>.
      </p>
    </section>
  </div>
</template>

<style scoped>
.group {
  margin-top: 1rem;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
}

.hint {
  color: var(--color-secondary);
  margin: 0;
  font-size: 13.5px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 13px;
  color: var(--color-secondary);
  width: 100%;
}

.account-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  width: 100%;
}

.head {
  width: 32px;
  height: 32px;
  border-radius: 6px;
}

.name {
  color: var(--color-contrast);
  font-weight: 600;
}

.active-badge {
  color: var(--color-brand);
  font-size: 12px;
  background: var(--color-brand-highlight);
  padding: 0.15rem 0.6rem;
  border-radius: 999px;
}

.small {
  padding: 0.3rem 0.7rem;
  font-size: 12.5px;
}

.danger {
  color: var(--color-red);
}

.device-code {
  background: var(--color-bg);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  align-items: flex-start;
}

.device-code p {
  margin: 0;
}

.code {
  font-size: 24px;
  letter-spacing: 4px;
  color: var(--color-brand);
  user-select: text;
}

.error {
  color: var(--color-red);
  font-size: 13px;
  margin: 0;
}

input[type='range'] {
  accent-color: var(--color-brand);
  padding: 0;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
