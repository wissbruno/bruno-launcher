use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
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
    #[serde(default)]
    version_number: String,
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
    #[serde(default)]
    sha512: String,
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

// ------------------------- Atualização de mods -------------------------

fn sha1_hex(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Consulta a API do Modrinth por hash SHA1 (batch): dado um conjunto de
/// hashes, devolve a versão correspondente de cada arquivo conhecido.
async fn versions_by_hashes(
    launcher: &Launcher,
    hashes: Vec<String>,
) -> Result<HashMap<String, ApiVersion>> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let body = serde_json::json!({ "hashes": hashes, "algorithm": "sha1" });
    Ok(launcher
        .http
        .post(format!("{LABRINTH}/version_files"))
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Lista os arquivos .jar da pasta mods de uma instância.
fn list_mod_files(launcher: &Launcher, instance_id: &str) -> Result<Vec<std::path::PathBuf>> {
    let dir = instance_dir(launcher, instance_id).join("mods");
    let mut out = Vec::new();
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().map(|e| e == "jar").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

#[derive(Serialize)]
pub struct ModUpdate {
    pub old_filename: String,
    pub new_filename: String,
    pub new_version: String,
    pub project_id: String,
}

/// Verifica quais mods instalados têm uma versão mais nova compatível com o
/// loader e a versão do jogo da instância.
#[tauri::command]
pub async fn check_mod_updates(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> Result<Vec<ModUpdate>> {
    let instance = load_instance(&launcher, &instance_id)?;
    let files = list_mod_files(&launcher, &instance_id)?;

    // hash de cada jar -> caminho
    let mut by_hash: HashMap<String, std::path::PathBuf> = HashMap::new();
    for path in &files {
        let bytes = std::fs::read(path)?;
        by_hash.insert(sha1_hex(&bytes), path.clone());
    }

    let current = versions_by_hashes(&launcher, by_hash.keys().cloned().collect()).await?;

    let mut updates = Vec::new();
    for (hash, version) in &current {
        let Some(path) = by_hash.get(hash) else { continue };
        let old_filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        // Última versão compatível do mesmo projeto
        let latest = match pick_version(&launcher, &version.project_id, &instance, "mod").await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if latest.id == version.id {
            continue; // já é a mais nova
        }
        let file = match primary_file(&latest) {
            Ok(f) => f,
            Err(_) => continue,
        };
        // Só conta como update se o arquivo é realmente diferente
        if file.filename == old_filename {
            continue;
        }
        updates.push(ModUpdate {
            old_filename,
            new_filename: file.filename.clone(),
            new_version: latest.version_number.clone(),
            project_id: version.project_id.clone(),
        });
    }
    Ok(updates)
}

/// Aplica todas as atualizações disponíveis: baixa as versões novas e remove
/// os arquivos antigos. Retorna os nomes dos mods atualizados.
#[tauri::command]
pub async fn apply_mod_updates<R: Runtime>(
    app: AppHandle<R>,
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> Result<Vec<String>> {
    let instance = load_instance(&launcher, &instance_id)?;
    let updates = check_mod_updates(launcher.clone(), instance_id.clone()).await?;
    let mods_dir = instance_dir(&launcher, &instance_id).join("mods");
    let total = updates.len() as u64;
    let mut done = 0u64;
    let mut updated = Vec::new();

    for up in &updates {
        emit_progress(&app, &instance_id, &format!("Atualizando {}...", up.new_filename), done, total, false);
        // Rebusca a versão para pegar a URL do arquivo
        let latest = pick_version(&launcher, &up.project_id, &instance, "mod").await?;
        let file = primary_file(&latest)?;
        let dest = mods_dir.join(&file.filename);
        download_file(&launcher, &file.url, &dest, Some(&file.hashes.sha1)).await?;
        // Remove o arquivo antigo (se o nome mudou)
        if up.old_filename != up.new_filename {
            std::fs::remove_file(mods_dir.join(&up.old_filename)).ok();
        }
        updated.push(up.new_filename.clone());
        done += 1;
    }
    emit_progress(&app, &instance_id, "Mods atualizados", total, total, true);
    Ok(updated)
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

// ------------------------- Exportar modpack (.mrpack) -------------------------

/// Exporta a instância como um arquivo .mrpack na pasta Downloads.
/// Mods que existem no Modrinth entram no índice (com URL de download);
/// o resto (configs, mods manuais, resourcepacks) vai em overrides/.
#[tauri::command]
pub async fn export_modpack(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> Result<String> {
    use std::io::Write;

    let instance = load_instance(&launcher, &instance_id)?;
    let game_dir = instance_dir(&launcher, &instance_id);

    // 1. Junta os arquivos de conteúdo e calcula os hashes
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    for folder in ["mods", "resourcepacks", "shaderpacks"] {
        let dir = game_dir.join(folder);
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let p = entry?.path();
                if p.is_file() {
                    candidates.push(p);
                }
            }
        }
    }
    let mut by_hash: HashMap<String, std::path::PathBuf> = HashMap::new();
    for path in &candidates {
        let bytes = std::fs::read(path)?;
        by_hash.insert(sha1_hex(&bytes), path.clone());
    }
    let known = versions_by_hashes(&launcher, by_hash.keys().cloned().collect()).await?;

    // 2. Monta o índice: arquivos conhecidos viram entradas com URL
    let mut index_files = Vec::new();
    let mut in_index: HashSet<std::path::PathBuf> = HashSet::new();
    for (hash, version) in &known {
        let Some(path) = by_hash.get(hash) else { continue };
        let Ok(file) = primary_file(version) else { continue };
        let rel = path
            .strip_prefix(&game_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        index_files.push(serde_json::json!({
            "path": rel,
            "hashes": { "sha1": file.hashes.sha1, "sha512": file.hashes.sha512 },
            "downloads": [file.url],
            "fileSize": size,
        }));
        in_index.insert(path.clone());
    }

    // 3. Dependências (loader + versão do jogo)
    let mut deps = serde_json::Map::new();
    deps.insert("minecraft".into(), instance.game_version.clone().into());
    if let Some(lv) = &instance.loader_version {
        let key = match instance.loader.as_str() {
            "fabric" => Some("fabric-loader"),
            "quilt" => Some("quilt-loader"),
            "forge" => Some("forge"),
            "neoforge" => Some("neoforge"),
            _ => None,
        };
        if let Some(k) = key {
            deps.insert(k.into(), lv.clone().into());
        }
    }

    let index = serde_json::json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0.0",
        "name": instance.name,
        "files": index_files,
        "dependencies": deps,
    });

    // 4. Escreve o .mrpack (zip) na pasta Downloads
    let downloads = dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| launcher.root.clone());
    std::fs::create_dir_all(&downloads)?;
    let safe_name: String = instance
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let out_path = downloads.join(format!("{safe_name}.mrpack"));
    let file = std::fs::File::create(&out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("modrinth.index.json", opts)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;

    // 5. Overrides: tudo que não entrou no índice (configs, mods manuais...)
    let override_dirs = ["mods", "resourcepacks", "shaderpacks", "config"];
    for folder in override_dirs {
        let dir = game_dir.join(folder);
        if !dir.exists() {
            continue;
        }
        add_overrides_recursive(&mut zip, &dir, &game_dir, &in_index, opts)?;
    }

    zip.finish()?;
    Ok(out_path.to_string_lossy().to_string())
}

fn add_overrides_recursive<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    game_dir: &Path,
    in_index: &HashSet<std::path::PathBuf>,
    opts: zip::write::FileOptions<()>,
) -> Result<()> {
    use std::io::Write;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            add_overrides_recursive(zip, &path, game_dir, in_index, opts)?;
        } else if !in_index.contains(&path) {
            let rel = path
                .strip_prefix(game_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(format!("overrides/{rel}"), opts)?;
            zip.write_all(&std::fs::read(&path)?)?;
        }
    }
    Ok(())
}
