<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';

/**
 * Renderiza o rosto de uma skin do Minecraft a partir do PNG (base64).
 * A face fica nos pixels (8,8)-(16,16) do skin; o "chapéu" (segunda camada)
 * fica em (40,8)-(48,8) e é desenhado por cima.
 */
const props = defineProps<{ pngBase64: string; size?: number }>();

const canvas = ref<HTMLCanvasElement | null>(null);

function render() {
  const el = canvas.value;
  if (!el || !props.pngBase64) return;
  const size = props.size ?? 80;
  el.width = size;
  el.height = size;
  const ctx = el.getContext('2d');
  if (!ctx) return;

  const img = new Image();
  img.onload = () => {
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, size, size);
    // Camada base do rosto
    ctx.drawImage(img, 8, 8, 8, 8, 0, 0, size, size);
    // Camada de chapéu/overlay (transparente na maioria das skins)
    ctx.drawImage(img, 40, 8, 8, 8, 0, 0, size, size);
  };
  img.src = `data:image/png;base64,${props.pngBase64}`;
}

onMounted(render);
watch(() => [props.pngBase64, props.size], render);
</script>

<template>
  <canvas ref="canvas" class="skin-face"></canvas>
</template>

<style scoped>
.skin-face {
  image-rendering: pixelated;
  border-radius: 8px;
  background: var(--color-bg);
}
</style>
