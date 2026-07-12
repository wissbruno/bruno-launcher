use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use futures::StreamExt;
use serde::Deserialize;
use tauri::{AppHandle, Runtime, State};

use crate::error::{AppError, Result};
use crate::instances::{instance_dir, load_instance, new_instance, Instance};
use crate::minecraft::download::download_file;
use crate::state::{emit_progress, Launcher};

const LABRINTH: &str = "https://api.modrinth.com/v2";

#[derive(Deserialize)]
struct ApiProject {
    id: String,
    title: String,
    project_type: String,
    icon_url: Option<String>,
}

#[derive(Deserialize, Clone)]
struct ApiVersion {
    id: String,
    project_id: String,
    files: Vec<ApiFile>,
    dependencies: Vec<ApiDependency>,
    loaders: Vec<String>,
    game_versions: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct ApiFile {
    url: String,
    filename: String,
    primary: bool,
    hashes: ApiHashes,
}

#[derive(Deserialize, Clone)]
struct ApiHashes {
    sha1: String,
}

#[derive(Deserialize, Clone)]
struct ApiDependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

async fn api_get<T: serde::de::DeserializeOwned>(launcher: &Launcher, path: &str) -> Result<T> {
    Ok(launcher
        .http
        .get(format!("{LABRINTH}{path}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn content_folder(project_type: &str) -> Result<&'static str> {
    match project_type {
        "mod" => Ok("mods"),
        "resourcepack" => Ok("resourcepacks"),
        "shader" => Ok("shaderpacks"),
        "datapack" => Ok("datapacks"),
        other => Err(AppError::msg(format!(
            "Tipo de projeto '{other}' não pode ser instalado em uma instância"
        ))),
    }
}

/// Escolhe a melhor versão de um projeto para a instância (loader e versão
/// do jogo compatíveis).
async fn pick_version(
    launcher: &Launcher,
    project_id: &str,
    instance: &Instance,
    project_type: &str,
) -> Result<ApiVersion> {
    // Mods dependem do loader; texturas/shaders/datapacks só da versão do jogo
    let loader_filter = if project_type == "mod" {
        format!("&loaders=[\"{}\"]", instance.loader)
    } else {
        String::new()
    };
    let path = format!(
        "/project/{project_id}/version?game_versions=[\"{}\"]{loader_filter}",
        instance.game_version
    );
    let versions: Vec<ApiVersion> = api_get(launcher, &path).await?;
    versions.into_iter().next().ok_or_else(|| {
        AppError::msg(format!(
            "Nenhuma versão compatível com {} {} encontrada",
            instance.loader, instance.game_version
        ))
    })
}

fn primary_file(version: &ApiVersion) -> Result<&ApiFile> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| AppError::msg("Versão sem arquivos"))
}

/// Instala um projeto Modrinth (mod/shader/textura/datapack) numa instância,
/// incluindo dependências obrigatórias de mods.
#[tauri::command]
pub async fn install_content<R: Runtime>(
    app: AppHandle<R>,
    launcher: State<'_, Launcher>,
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<Vec<String>> {
    install_content_inner(&app, &launcher, instance_id, project_id, version_id).await
}

pub async fn install_content_inner<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<Vec<String>> {
    let instance = load_instance(launcher, &instance_id)?;
    let mut installed: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    // Fila de (project_id, version_id opcional) para resolver dependências
    let mut queue: Vec<(String, Option<String>)> = vec![(project_id, version_id)];

    while let Some((pid, vid)) = queue.pop() {
        if !visited.insert(pid.clone()) {
            continue;
        }
        let project: ApiProject = api_get(&launcher, &format!("/project/{pid}")).await?;
        if project.project_type == "modpack" {
            return Err(AppError::msg(
                "Modpacks criam uma instância própria — use instalar modpack",
            ));
        }
        let folder = content_folder(&project.project_type)?;
        let version: ApiVersion = match &vid {
            Some(v) => api_get(&launcher, &format!("/version/{v}")).await?,
            None => pick_version(&launcher, &pid, &instance, &project.project_type).await?,
        };
        let file = primary_file(&version)?.clone();

        emit_progress(&app, &instance_id, &format!("Baixando {}...", file.filename), 0, 1, false);
        let dest = instance_dir(&launcher, &instance_id)
            .join(folder)
            .join(&file.filename);
        download_file(&launcher, &file.url, &dest, Some(&file.hashes.sha1)).await?;
        installed.push(file.filename.clone());

        // Dependências obrigatórias (só para mods)
        if project.project_type == "mod" {
            for dep in &version.dependencies {
                if dep.dependency_type == "required" {
                    if let Some(dep_pid) = &dep.project_id {
                        queue.push((dep_pid.clone(), dep.version_id.clone()));
                    } else if let Some(dep_vid) = &dep.version_id {
                        let v: ApiVersion =
                            api_get(&launcher, &format!("/version/{dep_vid}")).await?;
                        queue.push((v.project_id.clone(), Some(v.id.clone())));
                    }
                }
            }
        }
    }

    emit_progress(&app, &instance_id, "Instalação concluída", 1, 1, true);
    Ok(installed)
}

// ------------------------- Modpacks (.mrpack) -------------------------

#[derive(Deserialize)]
struct MrpackIndex {
    #[serde(default)]
    name: Option<String>,
    dependencies: std::collections::HashMap<String, String>,
    files: Vec<MrpackFile>,
}

#[derive(Deserialize)]
struct MrpackFile {
    path: String,
    hashes: ApiHashes,
    downloads: Vec<String>,
    #[serde(default)]
    env: Option<MrpackEnv>,
}

#[derive(Deserialize)]
struct MrpackEnv {
    #[serde(default)]
    client: Option<String>,
}

fn safe_relative_path(path: &str) -> Result<&Path> {
    let p = Path::new(path);
    if p.is_absolute()
        || path.contains("..")
        || path.contains(':')
        || path.starts_with('/')
        || path.starts_with('\\')
    {
        return Err(AppError::msg(format!("Caminho suspeito no modpack: {path}")));
    }
    Ok(p)
}

/// Instala um modpack do Modrinth criando uma instância nova a partir do
/// arquivo .mrpack (index + overrides).
#[tauri::command]
pub async fn install_modpack<R: Runtime>(
    app: AppHandle<R>,
    launcher: State<'_, Launcher>,
    project_id: String,
    version_id: Option<String>,
) -> Result<Instance> {
    let project: ApiProject = api_get(&launcher, &format!("/project/{project_id}")).await?;
    if project.project_type != "modpack" {
        return Err(AppError::msg("Este projeto não é um modpack"));
    }
    let version: ApiVersion = match &version_id {
        Some(v) => api_get(&launcher, &format!("/version/{v}")).await?,
        None => {
            let versions: Vec<ApiVersion> =
                api_get(&launcher, &format!("/project/{project_id}/version")).await?;
            versions
                .into_iter()
                .next()
                .ok_or_else(|| AppError::msg("Modpack sem versões"))?
        }
    };
    let file = primary_file(&version)?.clone();

    // 1. Baixa o .mrpack
    emit_progress(&app, &project.id, "Baixando modpack...", 0, 1, false);
    let tmp = launcher.meta_dir().join(format!("{}.mrpack", version.id));
    download_file(&launcher, &file.url, &tmp, Some(&file.hashes.sha1)).await?;

    // 2. Lê o índice
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&tmp)?)?;
    let index: MrpackIndex = {
        let mut entry = zip.by_name("modrinth.index.json")?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        serde_json::from_str(&text)?
    };

    let game_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| AppError::msg("Modpack sem versão do Minecraft"))?;
    let (loader, loader_version) = if let Some(v) = index.dependencies.get("fabric-loader") {
        ("fabric", Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("quilt-loader") {
        ("quilt", Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("forge") {
        ("forge", Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("neoforge") {
        ("neoforge", Some(v.clone()))
    } else {
        ("vanilla", None)
    };

    // 3. Cria a instância
    let display_name = index.name.clone().unwrap_or_else(|| project.title.clone());
    let instance = new_instance(
        &launcher,
        &display_name,
        &game_version,
        loader,
        loader_version,
        project.icon_url.clone(),
        Some(project.id.clone()),
    )?;
    let game_dir = instance_dir(&launcher, &instance.id);

    // 4. Baixa os arquivos do índice
    let files: Vec<&MrpackFile> = index
        .files
        .iter()
        .filter(|f| {
            f.env
                .as_ref()
                .and_then(|e| e.client.as_deref())
                .map(|c| c != "unsupported")
                .unwrap_or(true)
        })
        .collect();
    let total = files.len() as u64;
    emit_progress(&app, &instance.id, "Baixando arquivos do modpack...", 0, total, false);

    for f in &files {
        safe_relative_path(&f.path)?;
    }
    let mut done = 0u64;
    let launcher_ref: &Launcher = &launcher;
    let futs: Vec<_> = files
        .iter()
        .map(|f| {
            let dest = game_dir.join(f.path.replace('/', "\\"));
            let url = f.downloads.first().cloned().unwrap_or_default();
            let sha1 = f.hashes.sha1.clone();
            let path = f.path.clone();
            async move {
                if url.is_empty() {
                    return Err(AppError::msg(format!("Arquivo sem URL: {path}")));
                }
                download_file(launcher_ref, &url, &dest, Some(&sha1)).await
            }
        })
        .collect();
    let mut stream = futures::stream::iter(futs).buffer_unordered(12);
    while let Some(r) = stream.next().await {
        r?;
        done += 1;
        if done % 5 == 0 || done == total {
            emit_progress(&app, &instance.id, "Baixando arquivos do modpack...", done, total, false);
        }
    }

    // 5. Extrai overrides/ e client-overrides/
    emit_progress(&app, &instance.id, "Aplicando overrides...", 0, 1, false);
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().to_string();
            if !name.starts_with(prefix) || entry.is_dir() {
                continue;
            }
            let rel = &name[prefix.len()..];
            if rel.is_empty() {
                continue;
            }
            safe_relative_path(rel)?;
            let dest = game_dir.join(rel.replace('/', "\\"));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }

    std::fs::remove_file(&tmp).ok();
    emit_progress(&app, &instance.id, "Modpack pronto — preparando o jogo...", 1, 1, true);
    Ok(instance)
}
