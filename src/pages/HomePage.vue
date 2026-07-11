<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { searchProjects, type SearchHit } from '../api/modrinth';
import ProjectCard from '../components/ProjectCard.vue';

interface Row {
  title: string;
  type: 'modpack' | 'mod' | 'shader' | 'resourcepack';
  hits: SearchHit[];
}

const rows = ref<Row[]>([
  { title: 'Modpacks populares', type: 'modpack', hits: [] },
  { title: 'Mods em destaque', type: 'mod', hits: [] },
  { title: 'Shaders incríveis', type: 'shader', hits: [] },
  { title: 'Pacotes de textura', type: 'resourcepack', hits: [] },
]);

const error = ref('');

onMounted(async () => {
  try {
    await Promise.all(
      rows.value.map(async (row) => {
        const res = await searchProjects({ projectType: row.type, index: 'follows', limit: 6 });
        row.hits = res.hits;
      }),
    );
  } catch (e) {
    error.value = String(e);
  }
});
</script>

<template>
  <div class="home">
    <h1>Início</h1>
    <p class="subtitle">Descubra o melhor conteúdo da comunidade Modrinth.</p>

    <p v-if="error" class="error">Erro ao carregar: {{ error }}</p>

    <section v-for="row in rows" :key="row.type" class="row-section">
      <div class="row-header">
        <h2>{{ row.title }}</h2>
        <router-link :to="`/browse/${row.type}`">Ver tudo →</router-link>
      </div>
      <div class="grid">
        <ProjectCard v-for="hit in row.hits" :key="hit.project_id" :hit="hit" />
        <template v-if="!row.hits.length && !error">
          <div v-for="i in 6" :key="i" class="skeleton"></div>
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.subtitle {
  color: var(--color-secondary);
  margin: 0.35rem 0 1.5rem;
}

.row-section {
  margin-bottom: 2rem;
}

.row-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.row-header h2 {
  font-size: 18px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 0.75rem;
}

.skeleton {
  height: 96px;
  border-radius: var(--radius-lg);
  background: var(--color-raised-bg);
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  50% {
    opacity: 0.5;
  }
}

.error {
  color: var(--color-red);
}
</style>
