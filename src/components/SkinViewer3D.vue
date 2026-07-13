<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import { SkinViewer, WalkingAnimation } from 'skinview3d';

/**
 * Visualizador 3D da skin (boneco completo) usando skinview3d (three.js).
 * Auto-rotaciona e permite arrastar com o mouse para girar.
 */
const props = defineProps<{
  pngBase64: string;
  variant?: 'classic' | 'slim';
  width?: number;
  height?: number;
}>();

const canvas = ref<HTMLCanvasElement | null>(null);
let viewer: SkinViewer | null = null;

function load() {
  if (!viewer || !props.pngBase64) return;
  viewer.loadSkin(`data:image/png;base64,${props.pngBase64}`, {
    model: props.variant === 'slim' ? 'slim' : 'default',
  });
}

onMounted(() => {
  if (!canvas.value) return;
  viewer = new SkinViewer({
    canvas: canvas.value,
    width: props.width ?? 200,
    height: props.height ?? 280,
  });
  viewer.autoRotate = true;
  viewer.autoRotateSpeed = 0.6;
  viewer.animation = new WalkingAnimation();
  viewer.animation.speed = 0.5;
  viewer.zoom = 0.85;
  load();
});

watch(() => [props.pngBase64, props.variant], load);

onBeforeUnmount(() => {
  viewer?.dispose();
  viewer = null;
});
</script>

<template>
  <canvas ref="canvas" class="viewer"></canvas>
</template>

<style scoped>
.viewer {
  cursor: grab;
  border-radius: var(--radius-md);
}
.viewer:active {
  cursor: grabbing;
}
</style>
