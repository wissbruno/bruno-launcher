<script setup lang="ts">
import { computed } from 'vue';
import type { Instance } from '../api/backend';

const props = defineProps<{ instance: Instance; size?: number }>();

const colors = ['#1bd96a', '#4f9cff', '#c78aff', '#ffa347', '#ff496e'];

const color = computed(() => {
  let hash = 0;
  for (const c of props.instance.id) hash = (hash * 31 + c.charCodeAt(0)) | 0;
  return colors[Math.abs(hash) % colors.length];
});

const px = computed(() => `${props.size ?? 56}px`);
const initial = computed(() => props.instance.name.charAt(0).toUpperCase());
</script>

<template>
  <img
    v-if="instance.icon_url"
    :src="instance.icon_url"
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
}

.placeholder {
  display: grid;
  place-items: center;
  font-weight: 800;
  font-size: 1.4rem;
}
</style>
