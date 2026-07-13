//! Galeria de skins local.
//!
//! Guarda uma coleção de skins (PNG) em `<root>/skins/`, com um índice em
//! `skins.json`. Funciona 100% offline: você monta sua coleção, pré-visualiza
//! e marca uma favorita. Quando há uma conta Microsoft ativa, dá para aplicar
//! a skin de verdade no perfil (reaproveitando `msauth::upload_skin`).

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{AppError, Result};
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSkin {
    pub id: String,
    pub name: String,
    /// "classic" ou "slim"
    pub variant: String,
    pub added: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SkinLibrary {
    skins: Vec<SavedSkin>,
    favorite: Option<String>,
}

fn skins_dir(launcher: &Launcher) -> std::path::PathBuf {
    launcher.root.join("skins")
}

fn load_library(launcher: &Launcher) -> SkinLibrary {
    std::fs::read_to_string(skins_dir(launcher).join("skins.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_library(launcher: &Launcher, lib: &SkinLibrary) -> Result<()> {
    let dir = skins_dir(launcher);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("skins.json"), serde_json::to_string_pretty(lib)?)?;
    Ok(())
}

fn png_path(launcher: &Launcher, id: &str) -> std::path::PathBuf {
    skins_dir(launcher).join(format!("{id}.png"))
}

#[derive(Serialize)]
pub struct SkinWithData {
    #[serde(flatten)]
    pub skin: SavedSkin,
    /// PNG em base64 para pré-visualização no frontend
    pub png_base64: String,
    pub favorite: bool,
}

#[tauri::command]
pub fn list_saved_skins(launcher: State<'_, Launcher>) -> Result<Vec<SkinWithData>> {
    let lib = load_library(&launcher);
    let mut out = Vec::new();
    for skin in &lib.skins {
        let bytes = std::fs::read(png_path(&launcher, &skin.id)).unwrap_or_default();
        out.push(SkinWithData {
            skin: skin.clone(),
            png_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
            favorite: lib.favorite.as_deref() == Some(skin.id.as_str()),
        });
    }
    Ok(out)
}

/// Verificação mínima de PNG: assinatura de 8 bytes.
fn is_png(bytes: &[u8]) -> bool {
    bytes.len() > 8 && bytes[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}

/// Salva bytes de PNG já obtidos como uma skin na galeria.
fn store_skin_bytes(
    launcher: &Launcher,
    name: &str,
    variant: &str,
    bytes: &[u8],
) -> Result<SavedSkin> {
    if variant != "classic" && variant != "slim" {
        return Err(AppError::msg("Variante deve ser 'classic' ou 'slim'"));
    }
    if !is_png(bytes) {
        return Err(AppError::msg("O arquivo não é um PNG válido"));
    }
    if bytes.len() > 512 * 1024 {
        return Err(AppError::msg("Arquivo muito grande para uma skin"));
    }

    let id = format!("skin-{}", chrono::Utc::now().timestamp_millis());
    std::fs::create_dir_all(skins_dir(launcher))?;
    std::fs::write(png_path(launcher, &id), bytes)?;

    let skin = SavedSkin {
        id: id.clone(),
        name: if name.trim().is_empty() { "Skin".into() } else { name.to_string() },
        variant: variant.to_string(),
        added: chrono::Utc::now().to_rfc3339(),
    };
    let mut lib = load_library(launcher);
    lib.skins.insert(0, skin.clone());
    if lib.favorite.is_none() {
        lib.favorite = Some(id);
    }
    save_library(launcher, &lib)?;
    Ok(skin)
}

#[tauri::command]
pub fn add_saved_skin(
    launcher: State<'_, Launcher>,
    name: String,
    variant: String,
    png_base64: String,
) -> Result<SavedSkin> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|_| AppError::msg("PNG inválido (base64)"))?;
    store_skin_bytes(&launcher, &name, &variant, &bytes)
}

/// Importa a skin de qualquer jogador do Minecraft pelo nick, usando a API
/// oficial da Mojang (perfil → textura → PNG). Detecta o modelo (slim/classic).
#[tauri::command]
pub async fn import_skin_from_player(
    launcher: State<'_, Launcher>,
    username: String,
) -> Result<SavedSkin> {
    let username = username.trim();
    if username.is_empty() {
        return Err(AppError::msg("Digite um nome de jogador"));
    }

    // 1. nick → UUID
    let profile: serde_json::Value = {
        let res = launcher
            .http
            .get(format!(
                "https://api.mojang.com/users/profiles/minecraft/{username}"
            ))
            .send()
            .await?;
        if res.status().as_u16() == 404 || res.status().as_u16() == 204 {
            return Err(AppError::msg(format!("Jogador '{username}' não encontrado")));
        }
        res.error_for_status()?.json().await?
    };
    let uuid = profile["id"]
        .as_str()
        .ok_or_else(|| AppError::msg("Perfil sem UUID"))?;
    let real_name = profile["name"].as_str().unwrap_or(username).to_string();

    // 2. UUID → propriedades (textures em base64)
    let session: serde_json::Value = launcher
        .http
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let textures_b64 = session["properties"]
        .as_array()
        .and_then(|props| props.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or_else(|| AppError::msg("Perfil sem texturas"))?;
    let textures: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(textures_b64)
            .map_err(|_| AppError::msg("Texturas inválidas"))?,
    )?;
    let skin_node = &textures["textures"]["SKIN"];
    let skin_url = skin_node["url"]
        .as_str()
        .ok_or_else(|| AppError::msg(format!("'{real_name}' não tem skin personalizada")))?;
    let variant = if skin_node["metadata"]["model"] == "slim" {
        "slim"
    } else {
        "classic"
    };

    // 3. baixa o PNG da skin
    let bytes = launcher
        .http
        .get(skin_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    store_skin_bytes(&launcher, &real_name, variant, &bytes)
}

/// Importa uma skin a partir da URL de um PNG (ex.: link de imagem de um site
/// de skins). O usuário informa o modelo.
#[tauri::command]
pub async fn import_skin_from_url(
    launcher: State<'_, Launcher>,
    name: String,
    variant: String,
    url: String,
) -> Result<SavedSkin> {
    let url = url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::msg("URL inválida"));
    }
    let bytes = launcher
        .http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let fallback_name = if name.trim().is_empty() { "Skin da web" } else { name.trim() };
    store_skin_bytes(&launcher, fallback_name, &variant, &bytes)
}

#[tauri::command]
pub fn delete_saved_skin(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let mut lib = load_library(&launcher);
    lib.skins.retain(|s| s.id != id);
    if lib.favorite.as_deref() == Some(id.as_str()) {
        lib.favorite = lib.skins.first().map(|s| s.id.clone());
    }
    save_library(&launcher, &lib)?;
    std::fs::remove_file(png_path(&launcher, &id)).ok();
    Ok(())
}

#[tauri::command]
pub fn set_favorite_skin(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let mut lib = load_library(&launcher);
    if lib.skins.iter().any(|s| s.id == id) {
        lib.favorite = Some(id);
        save_library(&launcher, &lib)?;
    }
    Ok(())
}

/// Aplica uma skin salva ao perfil da conta ativa (exige login Microsoft).
#[tauri::command]
pub async fn apply_saved_skin(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let lib = load_library(&launcher);
    let skin = lib
        .skins
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::msg("Skin não encontrada"))?;
    let bytes = std::fs::read(png_path(&launcher, &id))?;
    let png_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    crate::msauth::upload_skin_inner(&launcher, png_base64, skin.variant.clone()).await
}
