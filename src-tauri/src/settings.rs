use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::Result;
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Memória máxima da JVM em MB
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    /// Overrides de caminho do java.exe por versão major ("21" -> caminho)
    #[serde(default)]
    pub java_overrides: HashMap<String, String>,
    /// Client ID do app registrado no Azure (necessário para login Microsoft)
    #[serde(default)]
    pub msa_client_id: Option<String>,
    /// Nome de jogador usado no modo offline
    #[serde(default = "default_offline_name")]
    pub offline_username: String,
}

fn default_memory() -> u32 {
    4096
}

fn default_offline_name() -> String {
    "Jogador".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory_mb: default_memory(),
            java_overrides: HashMap::new(),
            msa_client_id: None,
            offline_username: default_offline_name(),
        }
    }
}

pub fn load_settings(launcher: &Launcher) -> Result<Settings> {
    let path = launcher.root.join("settings.json");
    if !path.exists() {
        return Ok(Settings::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?).unwrap_or_default())
}

pub fn store_settings(launcher: &Launcher, settings: &Settings) -> Result<()> {
    std::fs::create_dir_all(&launcher.root)?;
    std::fs::write(
        launcher.root.join("settings.json"),
        serde_json::to_string_pretty(settings)?,
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(launcher: State<'_, Launcher>) -> Result<Settings> {
    load_settings(&launcher)
}

#[tauri::command]
pub fn set_settings(launcher: State<'_, Launcher>, settings: Settings) -> Result<Settings> {
    store_settings(&launcher, &settings)?;
    Ok(settings)
}
