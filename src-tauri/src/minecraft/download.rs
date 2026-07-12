use std::path::{Path, PathBuf};

use futures::StreamExt;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Runtime};
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, Result};
use crate::state::{emit_progress, Launcher};

use super::manifest::{maven_to_path, rules_allow, AssetIndexRef, Library, VersionJson, RESOURCES_URL};

fn sha1_of_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

/// Baixa `url` para `dest`, pulando se o arquivo já existe com o hash certo.
/// Verifica o sha1 após o download quando fornecido.
pub async fn download_file(
    launcher: &Launcher,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<()> {
    if dest.exists() {
        match sha1 {
            Some(expected) => {
                if sha1_of_file(dest)?.eq_ignore_ascii_case(expected) {
                    return Ok(());
                }
            }
            None => {
                if dest.metadata()?.len() > 0 {
                    return Ok(());
                }
            }
        }
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut last_err: Option<AppError> = None;
    for _attempt in 0..3 {
        match try_download(launcher, url, dest).await {
            Ok(()) => {
                if let Some(expected) = sha1 {
                    let actual = sha1_of_file(dest)?;
                    if !actual.eq_ignore_ascii_case(expected) {
                        last_err = Some(AppError::msg(format!(
                            "Hash inválido em {url}: esperado {expected}, obtido {actual}"
                        )));
                        continue;
                    }
                }
                return Ok(());
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| AppError::msg("Download falhou")))
}

async fn try_download(launcher: &Launcher, url: &str, dest: &Path) -> Result<()> {
    let res = launcher.http.get(url).send().await?.error_for_status()?;
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}

/// Uma biblioteca resolvida: de onde baixar e onde salvar.
pub struct ResolvedLibrary {
    /// Coordenada maven (group:artifact:version)
    pub name: String,
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    /// true quando é um jar de natives que precisa ser extraído
    pub native: bool,
}

/// Resolve as bibliotecas aplicáveis ao Windows, incluindo natives antigas
/// (classifiers) e bibliotecas maven do Fabric/Quilt.
pub fn resolve_libraries(launcher: &Launcher, libraries: &[Library]) -> Result<Vec<ResolvedLibrary>> {
    let lib_dir = launcher.libraries_dir();
    let mut out = Vec::new();
    for lib in libraries {
        if !rules_allow(&lib.rules) {
            continue;
        }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let rel = match &artifact.path {
                    Some(p) => p.clone(),
                    None => maven_to_path(&lib.name)?,
                };
                out.push(ResolvedLibrary {
                    name: lib.name.clone(),
                    url: artifact.url.clone(),
                    path: lib_dir.join(rel.replace('/', "\\")),
                    sha1: artifact.sha1.clone(),
                    native: false,
                });
            }
            // Natives antigas (<= 1.18): classifier tipo "natives-windows"
            if let (Some(natives), Some(classifiers)) = (&lib.natives, &downloads.classifiers) {
                if let Some(classifier) = natives.get("windows") {
                    let classifier = classifier.replace("${arch}", "64");
                    if let Some(artifact) = classifiers.get(&classifier) {
                        let rel = artifact
                            .path
                            .clone()
                            .unwrap_or_else(|| format!("natives/{}.jar", lib.name.replace(':', "-")));
                        out.push(ResolvedLibrary {
                            name: format!("{}:{}", lib.name, classifier),
                            url: artifact.url.clone(),
                            path: lib_dir.join(rel.replace('/', "\\")),
                            sha1: artifact.sha1.clone(),
                            native: true,
                        });
                    }
                }
            }
        } else if let Some(base) = &lib.url {
            // Biblioteca maven simples (Fabric/Quilt)
            let rel = maven_to_path(&lib.name)?;
            let base = base.trim_end_matches('/');
            out.push(ResolvedLibrary {
                name: lib.name.clone(),
                url: format!("{base}/{rel}"),
                path: lib_dir.join(rel.replace('/', "\\")),
                sha1: None,
                native: false,
            });
        }
    }
    Ok(out)
}

/// Baixa todas as bibliotecas e extrai as natives no diretório indicado.
pub async fn download_libraries<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    progress_id: &str,
    libraries: &[Library],
    natives_dir: &Path,
) -> Result<()> {
    let resolved = resolve_libraries(launcher, libraries)?;
    let total = resolved.len() as u64;
    let mut done = 0u64;
    emit_progress(app, progress_id, "Baixando bibliotecas...", 0, total, false);

    let futs: Vec<_> = resolved
        .iter()
        .map(|lib| {
            let url = lib.url.clone();
            let path = lib.path.clone();
            let sha1 = lib.sha1.clone();
            async move {
                if url.is_empty() {
                    // Biblioteca gerada localmente (instalador do Forge/NeoForge)
                    if path.exists() {
                        return Ok(());
                    }
                    return Err(crate::error::AppError::msg(format!(
                        "Biblioteca local não encontrada (rode a instalação de novo): {}",
                        path.display()
                    )));
                }
                download_file(launcher, &url, &path, sha1.as_deref()).await
            }
        })
        .collect();
    let mut stream = futures::stream::iter(futs).buffer_unordered(16);
    while let Some(r) = stream.next().await {
        r?;
        done += 1;
        if done % 8 == 0 || done == total {
            emit_progress(app, progress_id, "Baixando bibliotecas...", done, total, false);
        }
    }

    // Extrai natives (versões antigas)
    std::fs::create_dir_all(natives_dir)?;
    for lib in resolved.iter().filter(|l| l.native) {
        let file = std::fs::File::open(&lib.path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("META-INF") || entry.is_dir() {
                continue;
            }
            if let Some(enclosed) = entry.enclosed_name() {
                let dest = natives_dir.join(enclosed);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out = std::fs::File::create(dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct AssetIndexJson {
    #[serde(default)]
    objects: std::collections::HashMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    virtual_: Option<bool>,
    #[serde(default, rename = "map_to_resources")]
    map_to_resources: Option<bool>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
}

/// Baixa o índice de assets e todos os objetos que faltam.
pub async fn download_assets<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    progress_id: &str,
    index: &AssetIndexRef,
) -> Result<()> {
    let assets_dir = launcher.assets_dir();
    let index_path = assets_dir.join("indexes").join(format!("{}.json", index.id));
    download_file(launcher, &index.url, &index_path, Some(&index.sha1)).await?;

    let parsed: AssetIndexJson = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;
    let objects_dir = assets_dir.join("objects");

    let total = parsed.objects.len() as u64;
    emit_progress(app, progress_id, "Baixando assets do jogo...", 0, total, false);

    let entries: Vec<(String, String)> = parsed
        .objects
        .iter()
        .map(|(name, obj)| (name.clone(), obj.hash.clone()))
        .collect();

    let mut done = 0u64;
    let futs: Vec<_> = entries
        .iter()
        .map(|(_, hash)| {
            let hash = hash.clone();
            let prefix = hash[..2].to_string();
            let dest = objects_dir.join(&prefix).join(&hash);
            let url = format!("{RESOURCES_URL}/{prefix}/{hash}");
            async move { download_file(launcher, &url, &dest, Some(&hash)).await }
        })
        .collect();
    let mut stream = futures::stream::iter(futs).buffer_unordered(24);
    while let Some(r) = stream.next().await {
        r?;
        done += 1;
        if done % 50 == 0 || done == total {
            emit_progress(app, progress_id, "Baixando assets do jogo...", done, total, false);
        }
    }

    // Versões muito antigas esperam os assets como árvore de arquivos
    if parsed.virtual_.unwrap_or(false) || parsed.map_to_resources.unwrap_or(false) {
        let virtual_dir = assets_dir.join("virtual").join(&index.id);
        for (name, obj) in &parsed.objects {
            let src = objects_dir.join(&obj.hash[..2]).join(&obj.hash);
            let dest = virtual_dir.join(name.replace('/', "\\"));
            if !dest.exists() {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dest)?;
            }
        }
    }
    Ok(())
}

/// Baixa o client.jar da versão vanilla.
pub async fn download_client(launcher: &Launcher, vanilla_id: &str, version: &VersionJson) -> Result<PathBuf> {
    let client = version
        .downloads
        .as_ref()
        .and_then(|d| d.client.as_ref())
        .ok_or_else(|| AppError::msg("Versão sem download de client"))?;
    let jar = launcher
        .versions_dir()
        .join(vanilla_id)
        .join(format!("{vanilla_id}.jar"));
    download_file(launcher, &client.url, &jar, client.sha1.as_deref()).await?;
    Ok(jar)
}
