use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, Result};
use crate::state::Launcher;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub sha1: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// JSON de uma versão (client.json). Os perfis do Fabric/Quilt usam o mesmo
/// formato, com `inheritsFrom` apontando para a versão vanilla.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    pub main_class: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Formato antigo (<= 1.12): string única de argumentos do jogo
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "type", default)]
    pub version_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Value>,
    #[serde(default)]
    pub jvm: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    #[serde(default)]
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downloads {
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// Base maven (bibliotecas do Fabric/Quilt vêm só com name + url)
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

/// Avalia as regras de SO. Regras com `features` são tratadas como não
/// satisfeitas (usadas para demo/resolução customizada, que não suportamos).
pub fn rules_allow(rules: &Option<Vec<Rule>>) -> bool {
    let Some(rules) = rules else { return true };
    let mut allowed = false;
    for rule in rules {
        if rule.features.is_some() {
            return false;
        }
        let os_match = match &rule.os {
            None => true,
            Some(os) => {
                let name_ok = os.name.as_deref().map(|n| n == "windows").unwrap_or(true);
                let arch_ok = os.arch.as_deref().map(|a| a == "x86_64" || a == "x64").unwrap_or(true);
                name_ok && arch_ok
            }
        };
        match (rule.action.as_str(), os_match) {
            ("allow", true) => allowed = true,
            ("disallow", true) => return false,
            _ => {}
        }
    }
    allowed
}

/// Converte coordenadas maven "group:artifact:version[:classifier]" no caminho
/// relativo dentro de libraries/.
pub fn maven_to_path(name: &str) -> Result<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return Err(AppError::msg(format!("Coordenada maven inválida: {name}")));
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    Ok(format!(
        "{group}/{artifact}/{version}/{artifact}-{version}{classifier}.jar"
    ))
}

pub async fn fetch_manifest(launcher: &Launcher) -> Result<VersionManifest> {
    let cache = launcher.meta_dir().join("version_manifest_v2.json");
    match launcher.http.get(VERSION_MANIFEST_URL).send().await {
        Ok(res) if res.status().is_success() => {
            let text = res.text().await?;
            std::fs::create_dir_all(launcher.meta_dir())?;
            std::fs::write(&cache, &text)?;
            Ok(serde_json::from_str(&text)?)
        }
        // Sem internet: usa o cache se existir
        _ if cache.exists() => Ok(serde_json::from_str(&std::fs::read_to_string(cache)?)?),
        Err(e) => Err(e.into()),
        Ok(res) => Err(AppError::msg(format!(
            "Manifest da Mojang retornou {}",
            res.status()
        ))),
    }
}

/// Busca (com cache em versions/<id>/<id>.json) o JSON de uma versão vanilla.
pub async fn fetch_version_json(launcher: &Launcher, id: &str) -> Result<VersionJson> {
    let dir = launcher.versions_dir().join(id);
    let cache = dir.join(format!("{id}.json"));
    if cache.exists() {
        if let Ok(v) = serde_json::from_str::<VersionJson>(&std::fs::read_to_string(&cache)?) {
            return Ok(v);
        }
    }
    let manifest = fetch_manifest(launcher).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| AppError::msg(format!("Versão do Minecraft '{id}' não existe")))?;
    let text = launcher
        .http
        .get(&entry.url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&cache, &text)?;
    Ok(serde_json::from_str(&text)?)
}

/// Mescla um perfil de mod loader (child, com inheritsFrom) com a versão
/// vanilla (parent): bibliotecas somadas, argumentos concatenados, o resto
/// vem do parent quando o child não define.
pub fn merge_versions(parent: VersionJson, child: VersionJson) -> VersionJson {
    let arguments = match (parent.arguments, child.arguments) {
        (Some(p), Some(c)) => Some(Arguments {
            game: [p.game, c.game].concat(),
            jvm: [p.jvm, c.jvm].concat(),
        }),
        (p, c) => c.or(p),
    };
    VersionJson {
        id: child.id,
        inherits_from: None,
        main_class: child.main_class.or(parent.main_class),
        arguments,
        minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
        java_version: child.java_version.or(parent.java_version),
        libraries: [child.libraries, parent.libraries].concat(),
        version_type: child.version_type.or(parent.version_type),
    }
}
