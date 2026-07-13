<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useLauncherStore } from '../stores/launcher';
import {
  listSavedSkins,
  addSavedSkin,
  importSkinFromPlayer,
  importSkinFromUrl,
  deleteSavedSkin,
  setFavoriteSkin,
  applySavedSkin,
  type SavedSkin,
} from '../api/backend';
import SkinFace from '../components/SkinFace.vue';

const store = useLauncherStore();
const skins = ref<SavedSkin[]>([]);
const variant = ref<'classic' | 'slim'>('classic');
const busy = ref(false);
const msg = ref('');
const error = ref('');

// Importação online
const playerNick = ref('');
const skinUrl = ref('');

async function importByNick() {
  if (!playerNick.value.trim()) return;
  busy.value = true;
  error.value = '';
  msg.value = '';
  try {
    const skin = await importSkinFromPlayer(playerNick.value.trim());
    await refresh();
    msg.value = `Skin de ${skin.name} importada!`;
    playerNick.value = '';
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function importByUrl() {
  if (!skinUrl.value.trim()) return;
  busy.value = true;
  error.value = '';
  msg.value = '';
  try {
    await importSkinFromUrl('', variant.value, skinUrl.value.trim());
    await refresh();
    msg.value = 'Skin importada da web!';
    skinUrl.value = '';
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function refresh() {
  try {
    skins.value = await listSavedSkins();
  } catch {
    /* navegador */
  }
}

onMounted(() => {
  store.init();
  refresh();
});

async function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  busy.value = true;
  error.value = '';
  msg.value = '';
  try {
    const buf = await file.arrayBuffer();
    const base64 = btoa(String.fromCharCode(...new Uint8Array(buf)));
    const name = file.name.replace(/\.png$/i, '');
    await addSavedSkin(name, variant.value, base64);
    await refresh();
    msg.value = 'Skin adicionada à galeria!';
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
    (event.target as HTMLInputElement).value = '';
  }
}

async function favorite(skin: SavedSkin) {
  await setFavoriteSkin(skin.id);
  await refresh();
}

async function remove(skin: SavedSkin) {
  if (!confirm(`Remover a skin "${skin.name}" da galeria?`)) return;
  await deleteSavedSkin(skin.id);
  await refresh();
}

async function apply(skin: SavedSkin) {
  error.value = '';
  msg.value = '';
  try {
    await applySavedSkin(skin.id);
    msg.value = `Skin "${skin.name}" aplicada ao seu perfil!`;
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <div class="skins">
    <div class="header-row">
      <div>
        <h1>Galeria de Skins</h1>
        <p class="subtitle">Monte sua coleção de skins e aplique quando quiser.</p>
      </div>
      <label class="upload-btn btn-brand" :class="{ disabled: busy || !store.isTauri }">
        {{ busy ? 'Enviando...' : '+ Adicionar skin (PNG)' }}
        <input type="file" accept="image/png" hidden :disabled="busy || !store.isTauri" @change="onFile" />
      </label>
    </div>

    <div class="variant-hint">
      Modelo ao adicionar:
      <button :class="{ sel: variant === 'classic' }" @click="variant = 'classic'">Clássico (Steve)</button>
      <button :class="{ sel: variant === 'slim' }" @click="variant = 'slim'">Fino (Alex)</button>
    </div>

    <div class="import-online card">
      <div class="import-row">
        <label>🎮 Importar do jogador</label>
        <div class="inputs">
          <input
            v-model="playerNick"
            placeholder="Nick de qualquer jogador (ex.: Notch)"
            :disabled="busy || !store.isTauri"
            @keyup.enter="importByNick"
          />
          <button :disabled="busy || !playerNick.trim()" @click="importByNick">Buscar</button>
        </div>
        <span class="tip">Pega a skin real de qualquer conta do Minecraft pela API oficial da Mojang.</span>
      </div>
      <div class="import-row">
        <label>🔗 Importar de uma URL (PNG)</label>
        <div class="inputs">
          <input
            v-model="skinUrl"
            placeholder="Cole o link do PNG de um site de skins"
            :disabled="busy || !store.isTauri"
            @keyup.enter="importByUrl"
          />
          <button :disabled="busy || !skinUrl.trim()" @click="importByUrl">Baixar</button>
        </div>
        <span class="tip">Ache uma skin no minecraftskins.com/namemc, copie o link da imagem PNG e cole aqui (usa o modelo selecionado acima).</span>
      </div>
    </div>

    <p v-if="!store.activeAccount" class="note card">
      💡 Você pode montar sua galeria agora. Para <b>aplicar</b> uma skin de verdade no jogo é
      preciso estar logado com a conta Microsoft (após a aprovação da Mojang). Em modo offline,
      o boneco continua Steve/Alex — é limitação do próprio Minecraft.
    </p>

    <p v-if="msg" class="ok">{{ msg }}</p>
    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="skins.length" class="grid">
      <article v-for="skin in skins" :key="skin.id" class="card skin-card" :class="{ fav: skin.favorite }">
        <SkinFace :png-base64="skin.png_base64" :size="88" />
        <div class="info">
          <h3>{{ skin.name }}</h3>
          <span class="variant">{{ skin.variant === 'slim' ? 'Fino' : 'Clássico' }}</span>
        </div>
        <div class="actions">
          <button
            class="star"
            :class="{ active: skin.favorite }"
            :title="skin.favorite ? 'Favorita' : 'Marcar como favorita'"
            @click="favorite(skin)"
          >
            {{ skin.favorite ? '★' : '☆' }}
          </button>
          <button
            v-if="store.activeAccount"
            class="btn-brand small"
            title="Aplicar ao perfil"
            @click="apply(skin)"
          >
            Aplicar
          </button>
          <button class="small danger" title="Remover" @click="remove(skin)">🗑</button>
        </div>
      </article>
    </div>

    <div v-else-if="store.isTauri" class="card empty">
      <h3>Galeria vazia</h3>
      <p>
        Adicione arquivos PNG de skin (64×64). Você pode baixar skins de sites como
        <b>namemc.com</b> ou <b>minecraftskins.com</b> e importar aqui.
      </p>
    </div>

    <p v-else class="note card">Galeria disponível apenas no app desktop.</p>
  </div>
</template>

<style scoped>
.header-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
}

.subtitle {
  color: var(--color-secondary);
  margin: 0.35rem 0 0;
}

.upload-btn {
  cursor: pointer;
  padding: 0.6rem 1.1rem;
  border-radius: var(--radius-md);
  font-weight: 700;
}

.upload-btn.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.variant-hint {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 13px;
  color: var(--color-secondary);
  margin-bottom: 1rem;
}

.variant-hint button {
  padding: 0.35rem 0.75rem;
  font-size: 12.5px;
}

.variant-hint button.sel {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.import-online {
  display: flex;
  gap: 2rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.import-row {
  flex: 1;
  min-width: 260px;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.import-row label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-contrast);
}

.import-row .inputs {
  display: flex;
  gap: 0.5rem;
}

.import-row .inputs input {
  flex: 1;
  min-width: 0;
}

.import-row .tip {
  font-size: 11.5px;
  color: var(--color-secondary);
}

.note {
  color: var(--color-secondary);
  margin-bottom: 1rem;
  line-height: 1.5;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.75rem;
}

.skin-card {
  display: flex;
  gap: 0.85rem;
  align-items: center;
}

.skin-card.fav {
  box-shadow: 0 0 0 2px var(--color-brand-highlight);
}

.info {
  flex: 1;
  min-width: 0;
}

.info h3 {
  font-size: 14.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.variant {
  font-size: 12px;
  color: var(--color-secondary);
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  align-items: flex-end;
}

.star {
  background: transparent;
  color: var(--color-orange);
  font-size: 18px;
  padding: 0.1rem 0.4rem;
}

.star.active {
  color: var(--color-orange);
}

.small {
  padding: 0.3rem 0.6rem;
  font-size: 12px;
}

.danger {
  color: var(--color-red);
}

.empty {
  text-align: center;
  padding: 3rem 2rem;
  max-width: 480px;
  margin: 3rem auto;
}

.empty p {
  color: var(--color-secondary);
}

.ok {
  color: var(--color-brand);
}

.error {
  color: var(--color-red);
}
</style>
