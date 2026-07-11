<script setup lang="ts">
import { useRouter } from 'vue-router';
import { formatCount, type SearchHit } from '../api/modrinth';

defineProps<{ hit: SearchHit }>();
const router = useRouter();
</script>

<template>
  <article class="project-card" @click="router.push(`/project/${hit.project_id}`)">
    <img v-if="hit.icon_url" :src="hit.icon_url" class="icon" alt="" loading="lazy" />
    <div v-else class="icon placeholder"></div>
    <div class="info">
      <h3 class="title">{{ hit.title }}</h3>
      <span class="author">por {{ hit.author }}</span>
      <p class="desc">{{ hit.description }}</p>
      <div class="stats">
        <span title="Downloads">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 3v12m0 0 4-4m-4 4-4-4M4 21h16" />
          </svg>
          {{ formatCount(hit.downloads) }}
        </span>
        <span title="Seguidores">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 21s-7.5-4.6-9.5-9A5.5 5.5 0 0 1 12 6.5 5.5 5.5 0 0 1 21.5 12c-2 4.4-9.5 9-9.5 9z" />
          </svg>
          {{ formatCount(hit.follows) }}
        </span>
      </div>
    </div>
  </article>
</template>

<style scoped>
.project-card {
  display: flex;
  gap: 0.85rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  padding: 0.85rem;
  cursor: pointer;
  transition: filter 0.15s ease;
  min-width: 0;
}

.project-card:hover {
  filter: brightness(1.2);
}

.icon {
  width: 64px;
  height: 64px;
  border-radius: var(--radius-md);
  flex-shrink: 0;
  object-fit: cover;
  background: var(--color-button-bg);
}

.info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.title {
  font-size: 15px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.author {
  font-size: 12px;
  color: var(--color-secondary);
}

.desc {
  margin: 2px 0;
  font-size: 12.5px;
  color: var(--color-base);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.stats {
  display: flex;
  gap: 0.9rem;
  font-size: 12px;
  color: var(--color-secondary);
  margin-top: auto;
}

.stats span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.stats svg {
  width: 14px;
  height: 14px;
}
</style>
