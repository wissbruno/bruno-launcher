use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::state::Launcher;

use super::manifest::VersionJson;

/// Metadados do Fabric (meta.fabricmc.net) e do Quilt (meta.quiltmc.org),
/// que compartilham o mesmo formato de API.
#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

fn meta_base(loader: &str) -> Result<&'static str> {
    match loader {
        "fabric" => Ok("https://meta.fabricmc.net/v2"),
        "quilt" => Ok("https://meta.quiltmc.org/v3"),
        other => Err(AppError::msg(format!(
            "Mod loader '{other}' não é suportado (vanilla, fabric, quilt, forge, neoforge)"
        ))),
    }
}

/// Lista as versões do loader disponíveis para uma versão do jogo
/// (estáveis primeiro).
pub async fn loader_versions(
    launcher: &Launcher,
    loader: &str,
    game_version: &str,
) -> Result<Vec<String>> {
    match loader {
        "forge" => return super::forge::forge_versions(launcher, game_version).await,
        "neoforge" => return super::forge::neoforge_versions(launcher, game_version).await,
        _ => {}
    }
    let base = meta_base(loader)?;
    let url = format!("{base}/versions/loader/{game_version}");
    let entries: Vec<LoaderEntry> = launcher
        .http
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut stable: Vec<String> = Vec::new();
    let mut unstable: Vec<String> = Vec::new();
    for e in entries {
        if e.loader.stable.unwrap_or(false) {
            stable.push(e.loader.version);
        } else {
            unstable.push(e.loader.version);
        }
    }
    stable.extend(unstable);
    if stable.is_empty() {
        return Err(AppError::msg(format!(
            "Nenhuma versão do {loader} disponível para o Minecraft {game_version}"
        )));
    }
    Ok(stable)
}

/// Busca o perfil de versão do loader (um VersionJson com inheritsFrom),
/// com cache em meta/.
pub async fn loader_profile(
    launcher: &Launcher,
    loader: &str,
    game_version: &str,
    loader_version: &str,
) -> Result<VersionJson> {
    let base = meta_base(loader)?;
    let cache = launcher
        .meta_dir()
        .join(format!("{loader}-{loader_version}-{game_version}.json"));
    if cache.exists() {
        if let Ok(v) = serde_json::from_str::<VersionJson>(&std::fs::read_to_string(&cache)?) {
            return Ok(v);
        }
    }
    let url = format!("{base}/versions/loader/{game_version}/{loader_version}/profile/json");
    let text = launcher
        .http
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    std::fs::create_dir_all(launcher.meta_dir())?;
    std::fs::write(&cache, &text)?;
    Ok(serde_json::from_str(&text)?)
}
