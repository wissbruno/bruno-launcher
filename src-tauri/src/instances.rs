use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{AppError, Result};
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub game_version: String,
    /// "vanilla" | "fabric" | "quilt"
    pub loader: String,
    pub loader_version: Option<String>,
    pub created: String,
    pub last_played: Option<String>,
    pub icon_url: Option<String>,
    /// id do projeto Modrinth, quando criada a partir de um modpack
    pub modpack: Option<String>,
    /// true depois que jogo/bibliotecas/assets foram baixados com sucesso
    #[serde(default)]
    pub installed: bool,
    /// tempo total de jogatina acumulado, em segundos
    #[serde(default)]
    pub playtime_seconds: u64,
    /// true quando há um ícone personalizado salvo em <instance>/icon.png
    #[serde(default)]
    pub custom_icon: bool,
    /// instâncias fixadas aparecem no topo da biblioteca
    #[serde(default)]
    pub pinned: bool,
}

pub fn instance_dir(launcher: &Launcher, id: &str) -> PathBuf {
    launcher.instances_dir().join(id)
}

pub fn load_instance(launcher: &Launcher, id: &str) -> Result<Instance> {
    let path = instance_dir(launcher, id).join("instance.json");
    let data = fs::read_to_string(&path)
        .map_err(|_| AppError::msg(format!("Instância '{id}' não encontrada")))?;
    Ok(serde_json::from_str(&data)?)
}

pub fn save_instance(launcher: &Launcher, instance: &Instance) -> Result<()> {
    let dir = instance_dir(launcher, &instance.id);
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("instance.json"),
        serde_json::to_string_pretty(instance)?,
    )?;
    Ok(())
}

fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    let mut out = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                out.push(c);
            }
            prev_dash = true;
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    if out.is_empty() {
        "instancia".into()
    } else {
        out
    }
}

pub fn new_instance(
    launcher: &Launcher,
    name: &str,
    game_version: &str,
    loader: &str,
    loader_version: Option<String>,
    icon_url: Option<String>,
    modpack: Option<String>,
) -> Result<Instance> {
    let base = slugify(name);
    let mut id = base.clone();
    let mut n = 1;
    while instance_dir(launcher, &id).exists() {
        n += 1;
        id = format!("{base}-{n}");
    }
    let instance = Instance {
        id,
        name: name.to_string(),
        game_version: game_version.to_string(),
        loader: loader.to_string(),
        loader_version,
        created: chrono::Utc::now().to_rfc3339(),
        last_played: None,
        icon_url,
        modpack,
        installed: false,
        playtime_seconds: 0,
        custom_icon: false,
        pinned: false,
    };
    save_instance(launcher, &instance)?;
    Ok(instance)
}

/// Soma segundos de jogatina a uma instância (chamado quando o jogo encerra).
pub fn add_playtime(launcher: &Launcher, id: &str, seconds: u64) -> Result<()> {
    if let Ok(mut instance) = load_instance(launcher, id) {
        instance.playtime_seconds += seconds;
        save_instance(launcher, &instance)?;
    }
    Ok(())
}

// ------------------------- Comandos Tauri -------------------------

pub fn list_instances_inner(launcher: &Launcher) -> Result<Vec<Instance>> {
    let dir = launcher.instances_dir();
    let mut out = Vec::new();
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().join("instance.json").exists() {
                if let Ok(i) = load_instance(launcher, &entry.file_name().to_string_lossy()) {
                    out.push(i);
                }
            }
        }
    }
    out.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(out)
}

#[tauri::command]
pub fn list_instances(launcher: State<'_, Launcher>) -> Result<Vec<Instance>> {
    list_instances_inner(&launcher)
}

#[tauri::command]
pub fn create_instance(
    launcher: State<'_, Launcher>,
    name: String,
    game_version: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<Instance> {
    new_instance(
        &launcher,
        &name,
        &game_version,
        &loader,
        loader_version,
        None,
        None,
    )
}

#[tauri::command]
pub fn delete_instance(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let dir = instance_dir(&launcher, &id);
    if dir.join("instance.json").exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

#[tauri::command]
pub fn rename_instance(launcher: State<'_, Launcher>, id: String, name: String) -> Result<Instance> {
    let mut instance = load_instance(&launcher, &id)?;
    instance.name = name;
    save_instance(&launcher, &instance)?;
    Ok(instance)
}

/// Duplica uma instância (copia mods, saves, configs — tudo menos o
/// instance.json, que é recriado com novo id e playtime zerado).
#[tauri::command]
pub fn duplicate_instance(launcher: State<'_, Launcher>, id: String) -> Result<Instance> {
    let original = load_instance(&launcher, &id)?;
    let copy = new_instance(
        &launcher,
        &format!("{} (cópia)", original.name),
        &original.game_version,
        &original.loader,
        original.loader_version.clone(),
        original.icon_url.clone(),
        original.modpack.clone(),
    )?;

    // Copia o conteúdo da pasta (mods, saves, resourcepacks...), exceto o json
    let src = instance_dir(&launcher, &id);
    let dst = instance_dir(&launcher, &copy.id);
    copy_dir_except(&src, &dst, "instance.json")?;

    Ok(copy)
}

fn copy_dir_except(src: &std::path::Path, dst: &std::path::Path, skip: &str) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy() == skip {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_dir_except(&from, &to, "")?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Define (ou remove) o ícone personalizado da instância. `png_base64` vazio
/// remove o ícone.
#[tauri::command]
pub fn set_instance_icon(
    launcher: State<'_, Launcher>,
    id: String,
    png_base64: String,
) -> Result<Instance> {
    use base64::Engine;
    let mut instance = load_instance(&launcher, &id)?;
    let icon = instance_dir(&launcher, &id).join("icon.png");
    if png_base64.trim().is_empty() {
        std::fs::remove_file(&icon).ok();
        instance.custom_icon = false;
    } else {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png_base64.trim())
            .map_err(|_| AppError::msg("Imagem inválida (base64)"))?;
        if bytes.len() > 1024 * 1024 {
            return Err(AppError::msg("Imagem muito grande (máx. 1 MB)"));
        }
        std::fs::write(&icon, &bytes)?;
        instance.custom_icon = true;
    }
    save_instance(&launcher, &instance)?;
    Ok(instance)
}

/// Retorna o ícone personalizado em base64 (para o frontend exibir).
#[tauri::command]
pub fn get_instance_icon(launcher: State<'_, Launcher>, id: String) -> Result<String> {
    use base64::Engine;
    let icon = instance_dir(&launcher, &id).join("icon.png");
    let bytes = std::fs::read(&icon)
        .map_err(|_| AppError::msg("Sem ícone personalizado"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Fixa/desafixa a instância (aparece no topo da biblioteca).
#[tauri::command]
pub fn set_instance_pinned(
    launcher: State<'_, Launcher>,
    id: String,
    pinned: bool,
) -> Result<Instance> {
    let mut instance = load_instance(&launcher, &id)?;
    instance.pinned = pinned;
    save_instance(&launcher, &instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn open_instance_folder(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let dir = instance_dir(&launcher, &id);
    fs::create_dir_all(&dir)?;
    tauri_plugin_opener::open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::msg(e.to_string()))
}

#[derive(Serialize)]
pub struct ContentFile {
    pub folder: String,
    pub filename: String,
    pub size: u64,
}

const CONTENT_FOLDERS: [&str; 4] = ["mods", "resourcepacks", "shaderpacks", "datapacks"];

#[tauri::command]
pub fn list_instance_content(launcher: State<'_, Launcher>, id: String) -> Result<Vec<ContentFile>> {
    let dir = instance_dir(&launcher, &id);
    let mut out = Vec::new();
    for folder in CONTENT_FOLDERS {
        let sub = dir.join(folder);
        if !sub.exists() {
            continue;
        }
        for entry in fs::read_dir(sub)? {
            let entry = entry?;
            if entry.path().is_file() {
                out.push(ContentFile {
                    folder: folder.to_string(),
                    filename: entry.file_name().to_string_lossy().to_string(),
                    size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn remove_instance_content(
    launcher: State<'_, Launcher>,
    id: String,
    folder: String,
    filename: String,
) -> Result<()> {
    if !CONTENT_FOLDERS.contains(&folder.as_str()) {
        return Err(AppError::msg("Pasta inválida"));
    }
    // Evita path traversal (../../)
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(AppError::msg("Nome de arquivo inválido"));
    }
    let path = instance_dir(&launcher, &id).join(folder).join(filename);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}
