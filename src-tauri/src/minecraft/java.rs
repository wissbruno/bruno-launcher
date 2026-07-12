use std::path::{Path, PathBuf};

use tauri::{AppHandle, Runtime};

use crate::error::{AppError, Result};
use crate::settings;
use crate::state::{emit_progress, Launcher};

use super::download::download_file;

/// Mapeia a versão major exigida pelo Minecraft para uma versão LTS
/// disponível no Adoptium (Temurin).
pub fn adoptium_major(required: u32) -> u32 {
    match required {
        0..=8 => 8,
        9..=11 => 11,
        12..=17 => 17,
        _ => 21,
    }
}

fn find_java_exe(dir: &Path) -> Option<PathBuf> {
    // O zip do Temurin extrai para uma subpasta tipo jdk-21.0.5+11-jre/
    let direct = dir.join("bin").join("java.exe");
    if direct.exists() {
        return Some(direct);
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let candidate = entry.path().join("bin").join("java.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Garante um runtime Java com a versão major exigida, baixando o Temurin
/// (Eclipse Adoptium) se necessário. Respeita override nas configurações.
pub async fn ensure_java<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    progress_id: &str,
    required_major: u32,
) -> Result<PathBuf> {
    let major = adoptium_major(required_major);

    // Override manual nas configurações?
    let s = settings::load_settings(launcher)?;
    if let Some(path) = s.java_overrides.get(&major.to_string()) {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    let dir = launcher.java_dir().join(major.to_string());
    if let Some(exe) = find_java_exe(&dir) {
        return Ok(exe);
    }

    emit_progress(
        app,
        progress_id,
        &format!("Baixando Java {major} (Temurin)..."),
        0,
        1,
        false,
    );
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/x64/jre/hotspot/normal/eclipse"
    );
    let zip_path = launcher.java_dir().join(format!("temurin-{major}.zip"));
    download_file(launcher, &url, &zip_path, None).await?;

    emit_progress(app, progress_id, &format!("Extraindo Java {major}..."), 0, 1, false);
    std::fs::create_dir_all(&dir)?;
    let file = std::fs::File::open(&zip_path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    zip.extract(&dir)?;
    std::fs::remove_file(&zip_path).ok();

    find_java_exe(&dir).ok_or_else(|| AppError::msg("java.exe não encontrado após extração"))
}
