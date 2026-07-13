<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue';
import { getInstanceIcon, type Instance } from '../api/backend';

const props = defineProps<{ instance: Instance; size?: number }>();

const colors = ['#1bd96a', '#4f9cff', '#c78aff', '#ffa347', '#ff496e'];

const color = computed(() => {
  let hash = 0;
  for (const c of props.instance.id) hash = (hash * 31 + c.charCodeAt(0)) | 0;
  return colors[Math.abs(hash) % colors.length];
});

const px = computed(() => `${props.size ?? 56}px`);
const initial = computed(() => props.instance.name.charAt(0).toUpperCase());

// Ícone personalizado (salvo em disco) carregado sob demanda como data URL
const customSrc = ref<string | null>(null);
watchEffect(async () => {
  customSrc.value = null;
  if (props.instance.custom_icon) {
    try {
      const b64 = await getInstanceIcon(props.instance.id);
      customSrc.value = `data:image/png;base64,${b64}`;
    } catch {
      customSrc.value = null;
    }
  }
});

const src = computed(() => customSrc.value ?? props.instance.icon_url);
</script>

<template>
  <img
    v-if="src"
    :src="src"
    class="icon"
    :style="{ width: px, height: px }"
    alt=""
  />
  <div v-else class="icon placeholder" :style="{ width: px, height: px, background: color + '33', color }">
    {{ initial }}
  </div>
</template>

<style scoped>
.icon {
  border-radius: var(--radius-md);
  object-fit: cover;
  flex-shrink: 0;
  image-rendering: auto;
}

.placeholder {
  display: grid;
  place-items: center;
  font-weight: 800;
  font-size: 1.4rem;
}
</style>
