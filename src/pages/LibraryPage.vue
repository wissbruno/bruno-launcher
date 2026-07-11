<script setup lang="ts">
// Biblioteca de instâncias — será conectada ao backend Rust (theseus-like)
// quando o gerenciamento de instâncias for implementado.
</script>

<template>
  <div class="library">
    <h1>Biblioteca</h1>
    <p class="subtitle">Suas instalações do Minecraft aparecerão aqui.</p>

    <div class="card empty-state">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z" />
      </svg>
      <h3>Nenhuma instância ainda</h3>
      <p>
        O gerenciamento de instâncias (criar instalações com Fabric, Forge, Quilt ou NeoForge e
        lançar o jogo) é o próximo grande marco — será implementado no backend Rust.
      </p>
      <button class="btn-brand" disabled>+ Criar instância (em breve)</button>
    </div>
  </div>
</template>

<style scoped>
.subtitle {
  color: var(--color-secondary);
  margin: 0.35rem 0 1.5rem;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 3rem 2rem;
  text-align: center;
  max-width: 480px;
  margin: 3rem auto;
}

.empty-state svg {
  width: 48px;
  height: 48px;
  color: var(--color-secondary);
}

.empty-state p {
  color: var(--color-secondary);
  font-size: 13.5px;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
