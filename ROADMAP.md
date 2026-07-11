# Roadmap — Réplica do Modrinth App

Projeto de aprendizado: réplica do [Modrinth App](https://github.com/modrinth/code) com a mesma
stack do original — **Tauri 2 + Rust** (backend) e **Vue 3 + TypeScript + Pinia** (frontend).

## Marcos

### ✅ 1. Ambiente e esqueleto
- [x] Rust (MSVC) + VS Build Tools instalados
- [x] Projeto Tauri 2 + Vue 3 + TS criado
- [x] Tema escuro no estilo Modrinth
- [x] Navegação (Início, Descobrir, Biblioteca, Configurações)

### ✅ 2. Navegador de conteúdo (API Modrinth)
- [x] Cliente da API `api.modrinth.com/v2` (busca, projeto, versões, tags)
- [x] Página inicial com destaques (modpacks, mods, shaders, texturas)
- [x] Busca com filtros: categoria, mod loader, versão do jogo, ordenação, paginação
- [x] Página de projeto: descrição (markdown), versões, galeria, download

### 🔲 3. Instâncias (backend Rust)
- [ ] Modelo de instância (pasta, versão do MC, mod loader) salvo em disco
- [ ] Comandos Tauri: criar/renomear/apagar/listar instâncias
- [ ] Tela de biblioteca conectada ao backend

### 🔲 4. Launcher Minecraft
- [ ] Baixar version manifest da Mojang (piston-meta.mojang.com)
- [ ] Baixar client.jar, bibliotecas e assets com verificação de hash
- [ ] Detecção/instalação de Java
- [ ] Montar a linha de comando e lançar o jogo (modo offline primeiro)
- [ ] Instaladores de mod loader: Fabric, Quilt, Forge, NeoForge

### 🔲 5. Instalação de conteúdo
- [ ] Botão "Instalar" baixa o mod/shader/textura para dentro de uma instância
- [ ] Resolução de dependências de mods
- [ ] Importar/exportar modpacks (.mrpack)
- [ ] Verificar atualizações de mods instalados

### 🔲 6. Conta Microsoft
- [ ] Registrar app no Azure + formulário de aprovação da Mojang (ação manual do usuário)
- [ ] Fluxo OAuth device code → token Xbox Live → XSTS → token Minecraft
- [ ] Perfil, skins e capas

### 🔲 7. Extras
- [ ] Fila de downloads com progresso
- [ ] Configurações persistentes (Java, memória, diretórios)
- [ ] Logs do jogo em tempo real
- [ ] Empacotar instalador (.msi/.exe)

## Referências
- Código-fonte oficial: https://github.com/modrinth/code (backend do launcher: `packages/app-lib`, "theseus")
- Documentação da API: https://docs.modrinth.com/api
- Formato .mrpack: https://docs.modrinth.com/docs/modpacks/format_definition/
- Launcher meta da Mojang: https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
- Auth Microsoft/Minecraft: https://learn.microsoft.com/en-us/gaming/gdk/docs/services/minecraft/
