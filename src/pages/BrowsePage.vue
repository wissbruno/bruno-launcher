<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue';
import { useRoute } from 'vue-router';
import {
  searchProjects,
  getCategories,
  getLoaders,
  getGameVersions,
  type SearchHit,
  type Category,
  type Loader,
  type GameVersion,
  type ProjectType,
  type SortIndex,
} from '../api/modrinth';
import ProjectCard from '../components/ProjectCard.vue';

const route = useRoute();

const typeLabels: Record<ProjectType, string> = {
  modpack: 'Modpacks',
  mod: 'Mods',
  resourcepack: 'Pacotes de textura',
  shader: 'Shaders',
  datapack: 'Data packs',
};

const projectType = computed(() => (route.params.type as ProjectType) || 'mod');

const query = ref('');
const sort = ref<SortIndex>('relevance');
const gameVersion = ref('');
const selectedCategories = ref<string[]>([]);
const selectedLoaders = ref<string[]>([]);
const page = ref(0);
const limit = 20;

const hits = ref<SearchHit[]>([]);
const totalHits = ref(0);
const loading = ref(false);
const error = ref('');

const allCategories = ref<Category[]>([]);
const allLoaders = ref<Loader[]>([]);
const gameVersions = ref<GameVersion[]>([]);

const visibleCategories = computed(() =>
  allCategories.value.filter((c) => c.project_type === (projectType.value === 'datapack' ? 'mod' : projectType.value) && c.header === 'categories'),
);

const visibleLoaders = computed(() =>
  allLoaders.value.filter((l) => l.supported_project_types.includes(projectType.value)),
);

const totalPages = computed(() => Math.ceil(totalHits.value / limit));

let searchToken = 0;

async function runSearch() {
  const token = ++searchToken;
  loading.value = true;
  error.value = '';
  try {
    const res = await searchProjects({
      query: query.value || undefined,
      projectType: projectType.value,
      categories: selectedCategories.value,
      loaders: selectedLoaders.value,
      gameVersion: gameVersion.value || undefined,
      index: sort.value,
      offset: page.value * limit,
      limit,
    });
    if (token !== searchToken) return;
    hits.value = res.hits;
    totalHits.value = res.total_hits;
  } catch (e) {
    if (token === searchToken) error.value = String(e);
  } finally {
    if (token === searchToken) loading.value = false;
  }
}

function toggle(list: string[], value: string) {
  const i = list.indexOf(value);
  if (i >= 0) list.splice(i, 1);
  else list.push(value);
}

let debounce: ReturnType<typeof setTimeout>;
watch(query, () => {
  clearTimeout(debounce);
  debounce = setTimeout(() => {
    page.value = 0;
    runSearch();
  }, 300);
});

watch([sort, gameVersion, selectedCategories, selectedLoaders], () => {
  page.value = 0;
  runSearch();
}, { deep: true });

watch(page, runSearch);

watch(projectType, () => {
  selectedCategories.value = [];
  selectedLoaders.value = [];
  page.value = 0;
  runSearch();
});

onMounted(async () => {
  runSearch();
  try {
    [allCategories.value, allLoaders.value, gameVersions.value] = await Promise.all([
      getCategories(),
      getLoaders(),
      getGameVersions(),
    ]);
  } catch {
    /* filtros ficam vazios, busca continua funcionando */
  }
});
</script>

<template>
  <div class="browse">
    <h1>{{ typeLabels[projectType] }}</h1>

    <nav class="type-tabs">
      <router-link
        v-for="(label, type) in typeLabels"
        :key="type"
        :to="`/browse/${type}`"
        class="tab"
        :class="{ active: type === projectType }"
      >
        {{ label }}
      </router-link>
    </nav>

    <div class="layout">
      <aside class="filters card">
        <h3>Filtros</h3>

        <template v-if="visibleLoaders.length">
          <h4>Mod loader</h4>
          <label v-for="loader in visibleLoaders" :key="loader.name" class="check">
            <input
              type="checkbox"
              :checked="selectedLoaders.includes(loader.name)"
              @change="toggle(selectedLoaders, loader.name)"
            />
            {{ loader.name }}
          </label>
        </template>

        <h4>Versão do jogo</h4>
        <select v-model="gameVersion">
          <option value="">Todas</option>
          <option v-for="v in gameVersions.filter((v) => v.version_type === 'release')" :key="v.version" :value="v.version">
            {{ v.version }}
          </option>
        </select>

        <template v-if="visibleCategories.length">
          <h4>Categorias</h4>
          <label v-for="cat in visibleCategories" :key="cat.name" class="check">
            <input
              type="checkbox"
              :checked="selectedCategories.includes(cat.name)"
              @change="toggle(selectedCategories, cat.name)"
            />
            {{ cat.name }}
          </label>
        </template>
      </aside>

      <section class="results">
        <div class="search-row">
          <input v-model="query" class="search-input" :placeholder="`Buscar ${typeLabels[projectType].toLowerCase()}...`" />
          <select v-model="sort">
            <option value="relevance">Relevância</option>
            <option value="downloads">Downloads</option>
            <option value="follows">Seguidores</option>
            <option value="newest">Mais recentes</option>
            <option value="updated">Atualizados</option>
          </select>
        </div>

        <p v-if="error" class="error">Erro na busca: {{ error }}</p>

        <div class="grid" :class="{ dim: loading }">
          <ProjectCard v-for="hit in hits" :key="hit.project_id" :hit="hit" />
        </div>

        <p v-if="!loading && !hits.length && !error" class="empty">Nenhum resultado encontrado.</p>

        <div v-if="totalPages > 1" class="pagination">
          <button :disabled="page === 0" @click="page--">← Anterior</button>
          <span>Página {{ page + 1 }} de {{ totalPages.toLocaleString('pt-BR') }}</span>
          <button :disabled="page + 1 >= totalPages" @click="page++">Próxima →</button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.type-tabs {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
  flex-wrap: wrap;
}

.tab {
  padding: 0.45rem 0.9rem;
  border-radius: 999px;
  background: var(--color-raised-bg);
  color: var(--color-base);
  font-weight: 500;
}

.tab.active {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.layout {
  display: grid;
  grid-template-columns: 220px 1fr;
  gap: 1rem;
  align-items: start;
}

.filters {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  max-height: calc(100vh - 220px);
  overflow-y: auto;
  position: sticky;
  top: 0;
}

.filters h4 {
  margin-top: 0.6rem;
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--color-secondary);
}

.check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 13px;
  cursor: pointer;
  text-transform: capitalize;
}

.check input {
  accent-color: var(--color-brand);
}

.search-row {
  display: flex;
  gap: 0.6rem;
  margin-bottom: 1rem;
}

.search-input {
  flex: 1;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 0.75rem;
  transition: opacity 0.15s;
}

.grid.dim {
  opacity: 0.5;
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin: 1.5rem 0;
}

.error {
  color: var(--color-red);
}

.empty {
  color: var(--color-secondary);
  text-align: center;
  margin-top: 2rem;
}
</style>
