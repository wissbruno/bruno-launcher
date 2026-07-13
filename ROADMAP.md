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

### ✅ 3. Instâncias (backend Rust)
- [x] Modelo de instância (pasta, versão do MC, mod loader) salvo em disco
- [x] Comandos Tauri: criar/renomear/apagar/listar instâncias + conteúdo
- [x] Tela de biblioteca conectada ao backend + página da instância

### ✅ 4. Launcher Minecraft
- [x] Version manifest da Mojang (piston-meta) com cache offline
- [x] Baixar client.jar, bibliotecas e assets com verificação sha1 e retry
- [x] Java automático por versão (Temurin/Adoptium), com override manual
- [x] Montar a linha de comando e lançar o jogo (modo offline)
- [x] Mod loaders: Fabric e Quilt (perfis via meta oficial)
- [x] Forge e NeoForge (instalador oficial rodado em modo headless)

### ✅ 5. Instalação de conteúdo
- [x] Botão "Instalar" escolhe a instância e baixa mod/shader/textura/datapack
- [x] Resolução de dependências obrigatórias de mods
- [x] Importar modpacks .mrpack (índice + overrides) criando instância
- [ ] Verificar atualizações de mods instalados (futuro)
- [ ] Exportar modpack (futuro)

### ✅ 6. Conta Microsoft (código pronto; requer aprovação da Mojang)
- [x] Fluxo OAuth device code → Xbox Live → XSTS → token Minecraft → perfil
- [x] Múltiplas contas, conta ativa, renovação automática de token
- [ ] **Ação manual**: registrar app no Azure + formulário de aprovação da Mojang
      e colar o Client ID nas Configurações — guia em docs/LOGIN-MICROSOFT.md
- [x] Skins: visualização e upload (classic/slim) para a conta ativa

### 🔶 7. Extras
- [x] Painel de progresso de downloads (eventos Rust → frontend)
- [x] Configurações persistentes (memória, nome offline, client ID, Java)
- [x] Logs do jogo em tempo real na página da instância
- [x] Instalador (.msi/.exe) via `npm run tauri build`
- [x] Contador de horas de jogatina por instância + total
- [x] Galeria de skins local (importar, preview de rosto, favoritar, aplicar)
- [x] Duplicar instância e ordenar biblioteca (recentes/mais jogadas/nome)
- [ ] Verificar atualizações de mods instalados (futuro)
- [ ] Exportar modpack (futuro)
- [ ] Adicionar amigos (depende de login online + API social limitada — futuro)
- [ ] Ícone personalizado do app (futuro — `npm run tauri icon <png>`)

## Referências
- Código-fonte oficial: https://github.com/modrinth/code (backend do launcher: `packages/app-lib`, "theseus")
- Documentação da API: https://docs.modrinth.com/api
- Formato .mrpack: https://docs.modrinth.com/docs/modpacks/format_definition/
- Launcher meta da Mojang: https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
- Auth Microsoft/Minecraft: https://learn.microsoft.com/en-us/gaming/gdk/docs/services/minecraft/
