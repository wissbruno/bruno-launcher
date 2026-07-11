<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import {
  getProject,
  getProjectVersions,
  formatCount,
  formatDate,
  type Project,
  type Version,
} from '../api/modrinth';

const route = useRoute();
const router = useRouter();

const project = ref<Project | null>(null);
const versions = ref<Version[]>([]);
const error = ref('');
const tab = ref<'description' | 'versions' | 'gallery'>('description');

const bodyHtml = computed(() =>
  project.value ? DOMPurify.sanitize(marked.parse(project.value.body, { async: false }) as string) : '',
);

onMounted(async () => {
  try {
    const id = route.params.id as string;
    project.value = await getProject(id);
    versions.value = await getProjectVersions(id);
  } catch (e) {
    error.value = String(e);
  }
});

function download(version: Version) {
  const file = version.files.find((f) => f.primary) ?? version.files[0];
  if (file) {
    // Por enquanto abre o download direto; quando o backend Rust existir,
    // isso passa a instalar o arquivo dentro de uma instância.
    window.open(file.url, '_blank');
  }
}

const typeBadge: Record<string, string> = {
  mod: 'Mod',
  modpack: 'Modpack',
  resourcepack: 'Pacote de textura',
  shader: 'Shader',
  datapack: 'Data pack',
};
</script>

<template>
  <div class="project-page">
    <button class="back" @click="router.back()">← Voltar</button>

    <p v-if="error" class="error">Erro ao carregar projeto: {{ error }}</p>

    <template v-if="project">
      <header class="card header">
        <img v-if="project.icon_url" :src="project.icon_url" class="icon" alt="" />
        <div v-else class="icon placeholder"></div>
        <div class="head-info">
          <div class="title-row">
            <h1>{{ project.title }}</h1>
            <span class="badge">{{ typeBadge[project.project_type] ?? project.project_type }}</span>
          </div>
          <p class="summary">{{ project.description }}</p>
          <div class="meta">
            <span>⬇ {{ formatCount(project.downloads) }} downloads</span>
            <span>♥ {{ formatCount(project.followers) }} seguidores</span>
            <span>Atualizado em {{ formatDate(project.updated) }}</span>
          </div>
          <div class="chips">
            <span v-for="l in project.loaders" :key="l" class="chip">{{ l }}</span>
            <span v-for="c in project.categories" :key="c" class="chip alt">{{ c }}</span>
          </div>
        </div>
        <button class="btn-brand install-btn" @click="versions.length && download(versions[0])">
          Instalar
        </button>
      </header>

      <nav class="tabs">
        <button :class="{ active: tab === 'description' }" @click="tab = 'description'">Descrição</button>
        <button :class="{ active: tab === 'versions' }" @click="tab = 'versions'">
          Versões ({{ versions.length }})
        </button>
        <button v-if="project.gallery.length" :class="{ active: tab === 'gallery' }" @click="tab = 'gallery'">
          Galeria
        </button>
      </nav>

      <section v-if="tab === 'description'" class="card body" v-html="bodyHtml"></section>

      <section v-else-if="tab === 'versions'" class="versions">
        <article v-for="v in versions" :key="v.id" class="card version-row">
          <div class="v-info">
            <strong>{{ v.name }}</strong>
            <span class="v-meta">
              <span class="chip" :class="v.version_type">{{ v.version_type }}</span>
              {{ v.game_versions.join(', ') }} · {{ v.loaders.join(', ') }} ·
              {{ formatDate(v.date_published) }} · ⬇ {{ formatCount(v.downloads) }}
            </span>
          </div>
          <button @click="download(v)">Baixar</button>
        </article>
      </section>

      <section v-else class="gallery">
        <figure v-for="(img, i) in project.gallery" :key="i" class="card">
          <img :src="img.url" :alt="img.title ?? ''" loading="lazy" />
          <figcaption v-if="img.title">{{ img.title }}</figcaption>
        </figure>
      </section>
    </template>
  </div>
</template>

<style scoped>
.back {
  margin-bottom: 1rem;
}

.header {
  display: flex;
  gap: 1rem;
  align-items: flex-start;
}

.icon {
  width: 96px;
  height: 96px;
  border-radius: var(--radius-lg);
  object-fit: cover;
  background: var(--color-button-bg);
  flex-shrink: 0;
}

.head-info {
  flex: 1;
  min-width: 0;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.badge {
  background: var(--color-button-bg);
  border-radius: 999px;
  padding: 0.2rem 0.7rem;
  font-size: 12px;
}

.summary {
  color: var(--color-base);
  margin: 0.4rem 0;
}

.meta {
  display: flex;
  gap: 1.25rem;
  font-size: 13px;
  color: var(--color-secondary);
  flex-wrap: wrap;
}

.chips {
  display: flex;
  gap: 0.4rem;
  margin-top: 0.6rem;
  flex-wrap: wrap;
}

.chip {
  background: var(--color-button-bg);
  border-radius: 999px;
  padding: 0.15rem 0.6rem;
  font-size: 12px;
  text-transform: capitalize;
}

.chip.alt {
  background: transparent;
  border: 1px solid var(--color-divider);
}

.chip.release { color: var(--color-brand); }
.chip.beta { color: var(--color-orange); }
.chip.alpha { color: var(--color-red); }

.install-btn {
  align-self: center;
  padding: 0.7rem 1.5rem;
}

.tabs {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}

.tabs button {
  background: transparent;
  color: var(--color-secondary);
}

.tabs button.active {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.body {
  user-select: text;
  line-height: 1.6;
  overflow-x: auto;
}

.body :deep(img) {
  max-width: 100%;
}

.body :deep(a) {
  color: var(--color-brand);
}

.body :deep(h1),
.body :deep(h2),
.body :deep(h3) {
  margin: 1rem 0 0.5rem;
}

.versions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1rem;
}

.v-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.v-info strong {
  color: var(--color-contrast);
}

.v-meta {
  font-size: 12.5px;
  color: var(--color-secondary);
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.gallery {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 0.75rem;
}

.gallery figure {
  margin: 0;
  padding: 0.5rem;
}

.gallery img {
  width: 100%;
  border-radius: var(--radius-md);
}

.gallery figcaption {
  padding: 0.5rem;
  font-size: 13px;
}

.error {
  color: var(--color-red);
}
</style>
