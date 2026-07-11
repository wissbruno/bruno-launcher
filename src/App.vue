<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

const navItems = [
  { name: 'home', path: '/', label: 'Início', icon: 'home' },
  { name: 'browse', path: '/browse/modpack', label: 'Descobrir conteúdo', icon: 'compass' },
  { name: 'library', path: '/library', label: 'Biblioteca', icon: 'library' },
];

function isActive(item: { name: string }) {
  return route.name === item.name;
}
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="logo" title="Réplica do Modrinth App">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M12 2 3 7v10l9 5 9-5V7l-9-5z" />
          <path d="M12 8v8M8 10l4-2 4 2" />
        </svg>
      </div>

      <nav class="nav">
        <button
          v-for="item in navItems"
          :key="item.name"
          class="nav-btn"
          :class="{ active: isActive(item) }"
          :title="item.label"
          @click="router.push(item.path)"
        >
          <svg v-if="item.icon === 'home'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 10.5 12 3l9 7.5" />
            <path d="M5 9.5V21h14V9.5" />
          </svg>
          <svg v-else-if="item.icon === 'compass'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="9" />
            <path d="m15.5 8.5-2 5-5 2 2-5 5-2z" />
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z" />
          </svg>
        </button>
      </nav>

      <div class="sidebar-bottom">
        <button
          class="nav-btn"
          :class="{ active: route.name === 'settings' }"
          title="Configurações"
          @click="router.push('/settings')"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.09a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.09a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.09a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        </button>
        <button class="avatar" title="Entrar com conta Microsoft (em breve)">?</button>
      </div>
    </aside>

    <main class="content">
      <router-view v-slot="{ Component }">
        <component :is="Component" :key="route.fullPath" />
      </router-view>
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
}

.sidebar {
  width: 64px;
  flex-shrink: 0;
  background: var(--color-raised-bg);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0.75rem 0;
  gap: 0.75rem;
}

.logo {
  width: 40px;
  height: 40px;
  color: var(--color-brand);
  display: grid;
  place-items: center;
  margin-bottom: 0.25rem;
}

.logo svg {
  width: 30px;
  height: 30px;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  flex: 1;
}

.nav-btn {
  width: 44px;
  height: 44px;
  padding: 0;
  display: grid;
  place-items: center;
  background: transparent;
  color: var(--color-secondary);
  border-radius: var(--radius-md);
}

.nav-btn svg {
  width: 22px;
  height: 22px;
}

.nav-btn:hover {
  background: var(--color-button-bg);
  color: var(--color-contrast);
}

.nav-btn.active {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.sidebar-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.avatar {
  width: 40px;
  height: 40px;
  padding: 0;
  border-radius: 50%;
  background: var(--color-button-bg);
  color: var(--color-secondary);
  font-weight: 700;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem 2rem;
}
</style>
