<script setup lang="ts">
import { useLauncherStore } from '../stores/launcher';

const store = useLauncherStore();
</script>

<template>
  <transition-group name="fade" tag="div" class="panel">
    <div v-for="p in store.activeProgress" :key="p.id" class="card item">
      <span class="msg">{{ p.message }}</span>
      <div class="bar">
        <div
          class="fill"
          :style="{ width: p.total > 0 ? `${Math.min(100, (p.current / p.total) * 100)}%` : '30%' }"
          :class="{ indeterminate: p.total <= 1 }"
        ></div>
      </div>
      <span v-if="p.total > 1" class="count">{{ p.current }} / {{ p.total }}</span>
    </div>
  </transition-group>
</template>

<style scoped>
.panel {
  position: fixed;
  right: 1rem;
  bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  z-index: 90;
  width: 320px;
}

.item {
  padding: 0.75rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.msg {
  font-size: 13px;
  color: var(--color-contrast);
}

.bar {
  height: 6px;
  border-radius: 3px;
  background: var(--color-button-bg);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--color-brand);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.fill.indeterminate {
  animation: slide 1.2s ease-in-out infinite;
}

@keyframes slide {
  0% {
    margin-left: 0;
  }
  50% {
    margin-left: 70%;
  }
  100% {
    margin-left: 0;
  }
}

.count {
  font-size: 11.5px;
  color: var(--color-secondary);
  align-self: flex-end;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s, transform 0.3s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
