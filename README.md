# Bruno's Launcher

A personal, open-source game launcher for Minecraft: Java Edition, built from
scratch as a **learning project** — inspired by the excellent
[Modrinth App](https://github.com/modrinth/code), using the same kind of stack:
**Tauri 2 + Rust** on the backend and **Vue 3 + TypeScript + Pinia** on the
frontend.

> 🇧🇷 Projeto de aprendizado: uma réplica funcional do Modrinth App para
> entender como um launcher de Minecraft funciona por dentro. Interface em
> português.

## Features

- 🔍 **Content browser** — search mods, modpacks, shaders, resource packs and
  data packs from the [Modrinth public API](https://docs.modrinth.com/api),
  with filters (mod loader, game version, category), markdown project pages,
  version lists and galleries
- 📦 **Instances** — create isolated Minecraft installations with any game
  version and loader: **Vanilla, Fabric, Quilt, Forge or NeoForge**
- ⬇️ **Full game installation** — downloads the client, libraries and assets
  from official Mojang servers with SHA-1 verification and retry; Java
  runtimes (Temurin/Adoptium) are downloaded automatically per game version
- 🔧 **Forge/NeoForge support** — runs the official installer headlessly
  (`--installClient`) against the launcher's data directory
- 🧩 **One-click content install** — installs mods into a compatible instance
  resolving required dependencies; imports `.mrpack` modpacks (index +
  overrides) as new instances
- 🚀 **Game launching** — builds the full JVM/game command line (modern
  `arguments` and legacy `minecraftArguments` formats), streams live game
  logs to the UI, offline mode supported
- 👤 **Microsoft sign-in** — complete OAuth 2.0 device code flow
  (MSA → Xbox Live → XSTS → Minecraft Services) with token refresh, multiple
  accounts and skin upload. Requires an Azure AppID approved by Mojang
  (see [docs/LOGIN-MICROSOFT.md](docs/LOGIN-MICROSOFT.md))
- 📊 Download progress panel, persistent settings (JVM memory, offline
  username, Java overrides)

## Tech

| Layer     | Stack |
|-----------|-------|
| Shell     | Tauri 2 (Rust) |
| Backend   | tokio, reqwest, serde, zip, sha1 |
| Frontend  | Vue 3, TypeScript, Vite, Pinia, vue-router |
| Data      | Modrinth API v2 · Mojang piston-meta · Fabric/Quilt/Forge/NeoForge metas · Adoptium API |

## Development

```bash
npm install
npm run tauri dev      # desktop app with hot reload
npm run tauri build    # release installers (.msi / .exe)
```

End-to-end tests (they really download and launch the game for 40 seconds):

```bash
cd src-tauri
cargo test --test e2e_launch --no-run
# copy tests/common-controls-v6.manifest next to the built test exe as
# "<exe>.manifest" (required for comctl32 v6 / TaskDialogIndirect)
cargo test --test e2e_launch -- --nocapture
```

Launcher data lives in `%APPDATA%\ModrinthReplica` (instances, versions,
libraries, assets, Java runtimes).

## Legal

This is an unofficial hobby project, not affiliated with Mojang, Microsoft or
Modrinth. It downloads game files exclusively from official Mojang servers,
authenticates through the official Microsoft/Minecraft APIs, verifies game
ownership, and does not bypass any security, licensing or authentication
checks. You need to own Minecraft: Java Edition to play online.

## License

[MIT](LICENSE)
