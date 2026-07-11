<script setup lang="ts">
// Configurações — por enquanto informativas; ganharão persistência
// (memória de Java, diretórios, tema) junto com o backend Rust.
</script>

<template>
  <div class="settings">
    <h1>Configurações</h1>

    <section class="card group">
      <h3>Conta</h3>
      <p>
        O login com conta Microsoft será adicionado na etapa de autenticação. Ele exige registrar
        um aplicativo no Azure e pedir aprovação da Mojang para a API do Minecraft.
      </p>
      <button disabled>Entrar com Microsoft (em breve)</button>
    </section>

    <section class="card group">
      <h3>Java e desempenho</h3>
      <p>Detecção de Java, memória alocada e argumentos da JVM serão configuráveis aqui.</p>
    </section>

    <section class="card group">
      <h3>Sobre</h3>
      <p>
        Réplica do Modrinth App feita para aprendizado, com Tauri 2, Vue 3 e Rust.
        Dados fornecidos pela <a href="https://docs.modrinth.com" target="_blank">API pública do Modrinth</a>.
      </p>
    </section>
  </div>
</template>

<style scoped>
.group {
  margin-top: 1rem;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: flex-start;
}

.group p {
  color: var(--color-secondary);
  margin: 0;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
