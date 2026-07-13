use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::error::{AppError, Result};
use crate::instances::{instance_dir, load_instance, save_instance, Instance};
use crate::settings;
use crate::state::{emit_progress, Launcher};

use super::download::{download_assets, download_client, download_libraries, resolve_libraries};
use super::java::ensure_java;
use super::loader::loader_profile;
use super::manifest::{fetch_manifest, fetch_version_json, merge_versions, rules_allow, VersionJson};

/// Resolve o VersionJson final da instância (vanilla, ou perfil do loader
/// mesclado com o vanilla).
async fn resolve_version<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    instance: &Instance,
) -> Result<VersionJson> {
    let vanilla = fetch_version_json(launcher, &instance.game_version).await?;
    if instance.loader == "vanilla" {
        return Ok(vanilla);
    }
    let loader_version = instance
        .loader_version
        .clone()
        .ok_or_else(|| AppError::msg("Instância com loader mas sem versão do loader"))?;
    let profile = match instance.loader.as_str() {
        "forge" | "neoforge" => {
            super::forge::ensure_profile(
                app,
                launcher,
                &instance.id,
                &instance.loader,
                &instance.game_version,
                &loader_version,
            )
            .await?
        }
        _ => loader_profile(launcher, &instance.loader, &instance.game_version, &loader_version).await?,
    };
    Ok(merge_versions(vanilla, profile))
}

fn natives_dir(launcher: &Launcher, vanilla_id: &str) -> PathBuf {
    launcher.versions_dir().join(vanilla_id).join("natives")
}

/// Baixa tudo que a instância precisa: client.jar, bibliotecas, assets e Java.
pub async fn install<R: Runtime>(app: &AppHandle<R>, launcher: &Launcher, id: &str) -> Result<()> {
    let mut instance = load_instance(launcher, id)?;
    let version = resolve_version(app, launcher, &instance).await?;

    emit_progress(app, id, "Baixando o Minecraft...", 0, 1, false);
    download_client(launcher, &instance.game_version, &version).await?;

    download_libraries(
        app,
        launcher,
        id,
        &version.libraries,
        &natives_dir(launcher, &instance.game_version),
    )
    .await?;

    if let Some(index) = &version.asset_index {
        download_assets(app, launcher, id, index).await?;
    }

    let major = version.java_version.as_ref().map(|j| j.major_version).unwrap_or(8);
    ensure_java(app, launcher, id, major).await?;

    // Pastas padrão da instância
    let dir = instance_dir(launcher, id);
    for sub in ["mods", "resourcepacks", "shaderpacks", "saves"] {
        std::fs::create_dir_all(dir.join(sub))?;
    }

    instance.installed = true;
    save_instance(launcher, &instance)?;
    emit_progress(app, id, "Instalação concluída", 1, 1, true);
    Ok(())
}

/// Dados de autenticação usados na linha de comando do jogo.
pub struct AuthSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: String,
    pub user_type: String,
}

impl AuthSession {
    pub fn offline(username: &str) -> Self {
        // Mesmo esquema do launcher vanilla offline: UUID v3 derivado do nome
        let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
        let mut bytes = digest.0;
        bytes[6] = (bytes[6] & 0x0f) | 0x30; // versão 3
        bytes[8] = (bytes[8] & 0x3f) | 0x80; // variante RFC 4122
        let uuid = hex::encode(bytes);
        Self {
            username: username.to_string(),
            uuid,
            access_token: "0".into(),
            xuid: String::new(),
            user_type: "legacy".into(),
        }
    }
}

fn dedup_key(name: &str) -> String {
    // group:artifact[:classifier] (ignora só a versão) — o perfil do loader
    // vem primeiro e vence. O classifier PRECISA entrar na chave: as natives
    // modernas (ex.: org.lwjgl:lwjgl:3.3.3:natives-windows) são entradas
    // separadas do jar principal e ambas vão no classpath.
    let parts: Vec<&str> = name.split(':').collect();
    match (parts.first(), parts.get(1), parts.get(3)) {
        (Some(g), Some(a), Some(c)) => format!("{g}:{a}:{c}"),
        (Some(g), Some(a), None) => format!("{g}:{a}"),
        _ => name.to_string(),
    }
}

fn expand_arguments(args: &[Value], vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Value::String(s) => out.push(substitute(s, vars)),
            Value::Object(obj) => {
                let rules: Option<Vec<super::manifest::Rule>> = obj
                    .get("rules")
                    .and_then(|r| serde_json::from_value(r.clone()).ok());
                if !rules_allow(&rules) {
                    continue;
                }
                match obj.get("value") {
                    Some(Value::String(s)) => out.push(substitute(s, vars)),
                    Some(Value::Array(items)) => {
                        for item in items {
                            if let Value::String(s) = item {
                                out.push(substitute(s, vars));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

fn substitute(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut s = template.to_string();
    for (key, value) in vars {
        s = s.replace(&format!("${{{key}}}"), value);
    }
    s
}

/// Monta a linha de comando e inicia o jogo. Logs são emitidos como eventos
/// "game-log" e a saída como "game-exit".
pub async fn launch<R: Runtime>(
    app: &AppHandle<R>,
    launcher: &Launcher,
    id: &str,
    auth: AuthSession,
) -> Result<u32> {
    let mut instance = load_instance(launcher, id)?;
    if !instance.installed {
        install(app, launcher, id).await?;
        instance = load_instance(launcher, id)?;
    }
    let version = resolve_version(app, launcher, &instance).await?;
    let s = settings::load_settings(launcher)?;

    let game_dir = instance_dir(launcher, id);
    let natives = natives_dir(launcher, &instance.game_version);
    let assets_root = launcher.assets_dir();
    let asset_index_id = version.assets.clone().unwrap_or_else(|| "legacy".into());

    // Classpath: bibliotecas (dedup por group:artifact — o perfil do loader
    // vem primeiro na lista mesclada e vence) + client.jar
    let resolved = resolve_libraries(launcher, &version.libraries)?;
    let mut seen = std::collections::HashSet::new();
    let mut classpath: Vec<String> = Vec::new();
    for lib in resolved.iter().filter(|l| !l.native) {
        if seen.insert(dedup_key(&lib.name)) {
            classpath.push(lib.path.to_string_lossy().to_string());
        }
    }
    let mut client_jar = launcher
        .versions_dir()
        .join(&instance.game_version)
        .join(format!("{}.jar", instance.game_version));
    // Forge/NeoForge: a -DignoreList do bootstraplauncher ignora o jar
    // "<id-do-perfil>.jar" no classpath (como no launcher oficial, que baixa
    // o client com esse nome). Com o nome vanilla, o Java criaria um módulo
    // automático que colide com o módulo "minecraft" remendado.
    if matches!(instance.loader.as_str(), "forge" | "neoforge") && version.id != instance.game_version {
        let profile_jar = launcher
            .versions_dir()
            .join(&version.id)
            .join(format!("{}.jar", version.id));
        if !profile_jar.exists() {
            if let Some(parent) = profile_jar.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&client_jar, &profile_jar)?;
        }
        client_jar = profile_jar;
    }
    classpath.push(client_jar.to_string_lossy().to_string());
    let classpath_str = classpath.join(";");

    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", auth.username.clone()),
        ("version_name", version.id.clone()),
        ("game_directory", game_dir.to_string_lossy().to_string()),
        ("assets_root", assets_root.to_string_lossy().to_string()),
        ("game_assets", assets_root.join("virtual").join(&asset_index_id).to_string_lossy().to_string()),
        ("assets_index_name", asset_index_id.clone()),
        ("auth_uuid", auth.uuid.clone()),
        ("auth_access_token", auth.access_token.clone()),
        ("auth_session", auth.access_token.clone()),
        ("auth_xuid", auth.xuid.clone()),
        ("clientid", String::new()),
        ("user_type", auth.user_type.clone()),
        ("user_properties", "{}".to_string()),
        ("version_type", version.version_type.clone().unwrap_or_else(|| "release".into())),
        ("natives_directory", natives.to_string_lossy().to_string()),
        ("launcher_name", "modrinth-replica".to_string()),
        ("launcher_version", "0.1.0".to_string()),
        ("classpath", classpath_str.clone()),
        // Usadas pelo Forge/NeoForge (module path via bootstraplauncher)
        ("library_directory", launcher.libraries_dir().to_string_lossy().to_string()),
        ("classpath_separator", ";".to_string()),
        ("resolution_width", "1280".to_string()),
        ("resolution_height", "720".to_string()),
    ]);

    let mut jvm_args: Vec<String> = vec![format!("-Xmx{}M", s.memory_mb)];
    let mut has_cp = false;
    if let Some(arguments) = &version.arguments {
        let expanded = expand_arguments(&arguments.jvm, &vars);
        has_cp = expanded.iter().any(|a| a == "-cp" || a.starts_with("-Djava.library.path"));
        jvm_args.extend(expanded);
    }
    if !has_cp {
        jvm_args.push(format!("-Djava.library.path={}", natives.to_string_lossy()));
        jvm_args.push("-cp".into());
        jvm_args.push(classpath_str.clone());
    }

    let main_class = version
        .main_class
        .clone()
        .ok_or_else(|| AppError::msg("Versão sem mainClass"))?;

    let game_args: Vec<String> = if let Some(arguments) = &version.arguments {
        expand_arguments(&arguments.game, &vars)
    } else if let Some(legacy) = &version.minecraft_arguments {
        legacy.split_whitespace().map(|a| substitute(a, &vars)).collect()
    } else {
        Vec::new()
    };

    let major = version.java_version.as_ref().map(|j| j.major_version).unwrap_or(8);
    let java = ensure_java(app, launcher, id, major).await?;

    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&jvm_args)
        .arg(&main_class)
        .args(&game_args)
        .current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);
    launcher.running.lock().unwrap().insert(
        id.to_string(),
        crate::state::RunningGame {
            pid,
            started: std::time::Instant::now(),
        },
    );

    instance.last_played = Some(chrono::Utc::now().to_rfc3339());
    save_instance(launcher, &instance)?;

    // Streaming de logs para o frontend
    if let Some(out) = child.stdout.take() {
        spawn_log_reader(app.clone(), id.to_string(), BufReader::new(out).lines());
    }
    if let Some(err) = child.stderr.take() {
        spawn_log_reader(app.clone(), id.to_string(), BufReader::new(err).lines());
    }

    let app2 = app.clone();
    let id2 = id.to_string();
    tauri::async_runtime::spawn(async move {
        let status = child.wait().await;
        let code = status.ok().and_then(|st| st.code()).unwrap_or(-1);
        let launcher = app2.state::<Launcher>();
        // Calcula a duração da sessão e acumula no playtime da instância
        let elapsed = launcher
            .running
            .lock()
            .unwrap()
            .remove(&id2)
            .map(|g| g.started.elapsed().as_secs())
            .unwrap_or(0);
        // Só conta se o jogo ficou aberto tempo razoável (evita contar crashes
        // instantâneos de inicialização)
        if elapsed >= 3 {
            let _ = crate::instances::add_playtime(&launcher, &id2, elapsed);
        }
        let _ = app2.emit(
            "game-exit",
            GameExit {
                id: id2,
                code,
                session_seconds: elapsed,
            },
        );
    });

    Ok(pid)
}

#[derive(Clone, Serialize)]
struct GameExit {
    id: String,
    code: i32,
    session_seconds: u64,
}

#[derive(Clone, Serialize)]
struct GameLog {
    id: String,
    line: String,
}

fn spawn_log_reader<R, S>(app: AppHandle<R>, id: String, mut lines: tokio::io::Lines<S>)
where
    R: Runtime,
    S: tokio::io::AsyncBufRead + Unpin + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("game-log", GameLog { id: id.clone(), line });
        }
    });
}

// ------------------------- Comandos Tauri -------------------------

#[derive(Serialize)]
pub struct GameVersionEntry {
    pub id: String,
    pub version_type: String,
}

#[tauri::command]
pub async fn get_game_versions(launcher: State<'_, Launcher>) -> Result<Vec<GameVersionEntry>> {
    let manifest = fetch_manifest(&launcher).await?;
    Ok(manifest
        .versions
        .into_iter()
        .map(|v| GameVersionEntry {
            id: v.id,
            version_type: v.version_type,
        })
        .collect())
}

#[tauri::command]
pub async fn get_loader_versions(
    launcher: State<'_, Launcher>,
    loader: String,
    game_version: String,
) -> Result<Vec<String>> {
    super::loader::loader_versions(&launcher, &loader, &game_version).await
}

#[tauri::command]
pub async fn prepare_instance<R: Runtime>(
    app: AppHandle<R>,
    launcher: State<'_, Launcher>,
    id: String,
) -> Result<()> {
    let result = install(&app, &launcher, &id).await;
    if let Err(e) = &result {
        emit_progress(&app, &id, &format!("Erro: {e}"), 0, 1, true);
    }
    result
}

#[tauri::command]
pub async fn launch_instance<R: Runtime>(
    app: AppHandle<R>,
    launcher: State<'_, Launcher>,
    id: String,
    username: Option<String>,
) -> Result<u32> {
    // Conta ativa (Microsoft) se houver; senão, modo offline
    let auth = match crate::msauth::active_session(&launcher).await {
        Ok(Some(session)) => session,
        _ => AuthSession::offline(username.as_deref().unwrap_or("Jogador")),
    };
    let result = launch(&app, &launcher, &id, auth).await;
    if let Err(e) = &result {
        emit_progress(&app, &id, &format!("Erro: {e}"), 0, 1, true);
    }
    result
}

#[tauri::command]
pub fn kill_instance(launcher: State<'_, Launcher>, id: String) -> Result<()> {
    let pid = launcher.running.lock().unwrap().get(&id).map(|g| g.pid);
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn get_running(launcher: State<'_, Launcher>) -> Result<Vec<String>> {
    Ok(launcher.running.lock().unwrap().keys().cloned().collect())
}
