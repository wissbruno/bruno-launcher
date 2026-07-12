use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::Result;

/// Estado global do launcher: diretórios de dados e cliente HTTP.
/// Layout em disco (compartilhado entre instâncias, como no Modrinth App):
///   <data>/instances/<id>/   — pasta de jogo de cada instância (mods, saves...)
///   <data>/versions/<id>/    — jsons e jars de versões do Minecraft
///   <data>/libraries/        — bibliotecas Java compartilhadas
///   <data>/assets/           — assets do jogo (índices + objects)
///   <data>/java/<major>/     — runtimes Java baixados
///   <data>/meta/             — caches de manifests
pub struct Launcher {
    pub root: PathBuf,
    pub http: reqwest::Client,
    /// PIDs dos jogos em execução, por id de instância.
    pub running: Mutex<HashMap<String, u32>>,
}

impl Launcher {
    pub fn new() -> Result<Self> {
        let root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ModrinthReplica");
        std::fs::create_dir_all(&root)?;
        let http = reqwest::Client::builder()
            .user_agent("modrinth-replica-aprendizado/0.1 (brunourw@gmail.com)")
            .build()?;
        Ok(Self {
            root,
            http,
            running: Mutex::new(HashMap::new()),
        })
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }
    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn java_dir(&self) -> PathBuf {
        self.root.join("java")
    }
    pub fn meta_dir(&self) -> PathBuf {
        self.root.join("meta")
    }
}

/// Evento de progresso enviado ao frontend (barra de downloads).
#[derive(Clone, Serialize)]
pub struct Progress<'a> {
    /// Identificador da operação (ex.: id da instância).
    pub id: &'a str,
    pub message: &'a str,
    pub current: u64,
    pub total: u64,
    pub done: bool,
}

pub fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    message: &str,
    current: u64,
    total: u64,
    done: bool,
) {
    let _ = app.emit(
        "progress",
        Progress {
            id,
            message,
            current,
            total,
            done,
        },
    );
}
