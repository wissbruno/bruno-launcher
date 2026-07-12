//! Suporte a Forge e NeoForge.
//!
//! Diferente do Fabric/Quilt (que publicam um "profile json" pronto), o
//! Forge e o NeoForge usam um INSTALADOR oficial com "processors" que
//! remendam o jar do jogo. Em vez de reimplementar esse pipeline, rodamos o
//! instalador em modo headless (`--installClient`) apontando para a nossa
//! pasta de dados — que usa o mesmo layout do launcher vanilla
//! (versions/, libraries/), exatamente o que o instalador espera.

use serde::Deserialize;
use tauri::{AppHandle, Runtime};

use crate::error::{AppError, Result};
use crate::state::{emit_progress, Launcher};

use super::download::download_file;
use super::java::ensure_java;
use super::manifest::VersionJson;

const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const NEOFORGE_METADATA: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

#[derive(Deserialize)]
struct ForgePromotions {
    promos: std::collections::HashMap<String, String>,
}

/// Versões do Forge para uma versão do jogo: recomendada primeiro, depois a
/// mais recente (quando diferem).
pub async fn forge_versions(launcher: &Launcher, game_version: &str) -> Result<Vec<String>> {
    let promos: ForgePromotions = launcher
        .http
        .get(FORGE_PROMOTIONS)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut out = Vec::new();
    if let Some(v) = promos.promos.get(&format!("{game_version}-recommended")) {
        out.push(v.clone());
    }
    if let Some(v) = promos.promos.get(&format!("{game_version}-latest")) {
        if !out.contains(v) {
            out.push(v.clone());
        }
    }
    if out.is_empty() {
        return Err(AppError::msg(format!(
            "Forge não tem build para o Minecraft {game_version} (headless funciona só em 1.13+)"
        )));
    }
    Ok(out)
}

/// Versões do NeoForge para uma versão do jogo. O esquema de versão do
/// NeoForge é "MAJOR.MINOR.PATCH" onde MAJOR.MINOR espelham o Minecraft
/// (MC 1.21.1 → 21.1.x; MC 1.21 → 21.0.x).
pub async fn neoforge_versions(launcher: &Launcher, game_version: &str) -> Result<Vec<String>> {
    let xml = launcher
        .http
        .get(NEOFORGE_METADATA)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let rest = game_version
        .strip_prefix("1.")
        .ok_or_else(|| AppError::msg("NeoForge só existe para Minecraft 1.20.2+"))?;
    let prefix = if rest.contains('.') {
        format!("{rest}.")
    } else {
        format!("{rest}.0.")
    };

    let mut stable: Vec<String> = Vec::new();
    let mut beta: Vec<String> = Vec::new();
    for line in xml.lines() {
        let line = line.trim();
        if let Some(v) = line
            .strip_prefix("<version>")
            .and_then(|s| s.strip_suffix("</version>"))
        {
            if v.starts_with(&prefix) {
                if v.contains("beta") {
                    beta.push(v.to_string());
                } else {
                    stable.push(v.to_string());
                }
            }
        }
    }
    // O metadata vem em ordem crescente; queremos as mais novas primeiro
    stable.reverse();
    beta.reverse();
    stable.extend(beta);
    if stable.is_empty() {
        return Err(AppError::msg(format!(
            "NeoForge não tem build para o Minecraft {game_version}"
        )));
    }
    Ok(stable)
}

/// Id do perfil que o instalador cria em versions/.
pub fn profile_id(loader: &str, game_version: &str, loader_version: &str) -> String {
    match loader {
        "forge" => format!("{game_version}-forge-{loader_version}"),
        _ => format!("neoforge-{loader_version}"),
    }
}

fn installer_url(loader: &str, game_version: &str, loader_version: &str) -> String {
    match loader {
        "forge" => format!(
            "{FORGE_MAVEN}/{game_version}-{loader_version}/forge-{game_version}-{loader_version}-installer.jar"
        ),
        _ => format!("{NEOFORGE_MAVEN}/{loader_version}/neoforge-{loader_version}-installer.jar"),
    }
}

/// Garante que o perfil do Forge/NeoForge está instalado (rodando o
/// instalador oficial headless na primeira vez) e retorna o VersionJson dele.
pub async fn ensure_profile<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    progress_id: &str,
    loader: &str,
    game_version: &str,
    loader_version: &str,
) -> Result<VersionJson> {
    let id = profile_id(loader, game_version, loader_version);
    let json_path = launcher.versions_dir().join(&id).join(format!("{id}.json"));
    if json_path.exists() {
        return Ok(serde_json::from_str(&std::fs::read_to_string(&json_path)?)?);
    }

    // 1. Baixa o instalador
    emit_progress(app, progress_id, &format!("Baixando instalador do {loader}..."), 0, 1, false);
    let installer = launcher
        .meta_dir()
        .join(format!("{loader}-{game_version}-{loader_version}-installer.jar"));
    download_file(launcher, &installer_url(loader, game_version, loader_version), &installer, None)
        .await?;

    // 2. O instalador exige um launcher_profiles.json no diretório alvo
    let profiles = launcher.root.join("launcher_profiles.json");
    if !profiles.exists() {
        std::fs::write(&profiles, "{\"profiles\":{}}")?;
    }

    // 3. Roda o instalador headless com o Java da versão do jogo
    let vanilla = super::manifest::fetch_version_json(launcher, game_version).await?;
    let major = vanilla.java_version.as_ref().map(|j| j.major_version).unwrap_or(8);
    let java = ensure_java(app, launcher, progress_id, major).await?;

    emit_progress(
        app,
        progress_id,
        &format!("Instalando {loader} {loader_version} (processors)..."),
        0,
        1,
        false,
    );
    let mut cmd = tokio::process::Command::new(&java);
    cmd.arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(&launcher.root)
        .current_dir(&launcher.root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = cmd.output().await?;

    if !output.status.success() || !json_path.exists() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stdout
            .lines()
            .chain(stderr.lines())
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(AppError::msg(format!(
            "Instalador do {loader} falhou:\n{tail}"
        )));
    }

    std::fs::remove_file(&installer).ok();
    Ok(serde_json::from_str(&std::fs::read_to_string(&json_path)?)?)
}
