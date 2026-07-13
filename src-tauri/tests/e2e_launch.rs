//! Teste fim-a-fim: cria uma instância Fabric, baixa TUDO (Java, client,
//! bibliotecas, assets) e lança o Minecraft em modo offline por ~40s.
//!
//! Usa o mesmo diretório de dados do app (%APPDATA%\ModrinthReplica), então
//! serve também para aquecer o cache antes do uso real.
//!
//! Rodar com: cargo test --test e2e_launch -- --nocapture

use modrinth_replica_lib::content;
use modrinth_replica_lib::instances;
use modrinth_replica_lib::minecraft::launch::{self, AuthSession};
use modrinth_replica_lib::minecraft::loader;
use modrinth_replica_lib::skins;
use modrinth_replica_lib::state::Launcher;

const GAME: &str = "1.21.1";

/// Busca (ou cria) a instância compartilhada dos testes.
async fn test_instance(launcher: &Launcher) -> instances::Instance {
    let existing = instances::list_instances_inner(launcher).expect("listar");
    if let Some(i) = existing.into_iter().find(|i| i.name == "Teste E2E Fabric") {
        return i;
    }
    let loaders = loader::loader_versions(launcher, "fabric", GAME)
        .await
        .expect("versões do fabric");
    instances::new_instance(
        launcher,
        "Teste E2E Fabric",
        GAME,
        "fabric",
        Some(loaders[0].clone()),
        None,
        None,
    )
    .expect("criar instância")
}

#[tokio::test(flavor = "multi_thread")]
async fn cria_instala_e_lanca_fabric() {
    let mock = tauri::test::mock_app();
    let app = mock.handle().clone();
    let launcher = Launcher::new().expect("launcher");
    // O callback de saída do jogo consulta o estado gerenciado
    use tauri::Manager;
    app.manage(Launcher::new().expect("launcher 2"));

    // 1-2. Instância de teste (reutilizada entre execuções)
    let instance = test_instance(&launcher).await;
    println!("[teste] instância: {} (fabric {:?})", instance.id, instance.loader_version);

    // 3. Instala (baixa client, bibliotecas, assets, Java) — pode demorar
    launch::install(&app, &launcher, &instance.id)
        .await
        .expect("instalar jogo");
    println!("[teste] instalação concluída");

    // 4. Lança offline e verifica que o processo sobrevive à inicialização
    let pid = launch::launch(&app, &launcher, &instance.id, AuthSession::offline("TesteBot"))
        .await
        .expect("lançar jogo");
    println!("[teste] jogo lançado, pid {pid}");
    assert!(pid > 0);

    tokio::time::sleep(std::time::Duration::from_secs(40)).await;

    let alive = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false);

    // Encerra o jogo independentemente do resultado
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F", "/T"])
        .output();

    assert!(alive, "o processo do Minecraft morreu nos primeiros 40s — veja os logs");
    println!("[teste] SUCESSO: Minecraft rodou 40s sem crashar");
}

/// Cria uma instância NeoForge, roda o instalador oficial headless, baixa
/// tudo e lança o jogo por 40s.
#[tokio::test(flavor = "multi_thread")]
async fn cria_instala_e_lanca_neoforge() {
    let mock = tauri::test::mock_app();
    let app = mock.handle().clone();
    let launcher = Launcher::new().expect("launcher");
    use tauri::Manager;
    app.manage(Launcher::new().expect("launcher 2"));

    let versions = loader::loader_versions(&launcher, "neoforge", GAME)
        .await
        .expect("versões do neoforge");
    assert!(!versions.is_empty());
    println!("[teste] neoforge {} para MC {GAME}", versions[0]);

    let existing = instances::list_instances_inner(&launcher).expect("listar");
    let instance = match existing.into_iter().find(|i| i.name == "Teste E2E NeoForge") {
        Some(i) => i,
        None => instances::new_instance(
            &launcher,
            "Teste E2E NeoForge",
            GAME,
            "neoforge",
            Some(versions[0].clone()),
            None,
            None,
        )
        .expect("criar instância"),
    };
    println!("[teste] instância: {}", instance.id);

    launch::install(&app, &launcher, &instance.id)
        .await
        .expect("instalar neoforge");
    println!("[teste] instalação concluída (instalador headless ok)");

    let pid = launch::launch(&app, &launcher, &instance.id, AuthSession::offline("TesteBot"))
        .await
        .expect("lançar jogo");
    println!("[teste] jogo lançado, pid {pid}");

    tokio::time::sleep(std::time::Duration::from_secs(40)).await;
    let alive = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false);
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F", "/T"])
        .output();
    assert!(alive, "o Minecraft com NeoForge morreu nos primeiros 40s — veja os logs");
    println!("[teste] SUCESSO: Minecraft + NeoForge rodou 40s sem crashar");
}

/// Confere que a jogatina é somada: lança o jogo, espera ~8s, mata, e
/// verifica que playtime_seconds aumentou (>= 5s).
#[tokio::test(flavor = "multi_thread")]
async fn contabiliza_playtime() {
    use tauri::Manager;
    let mock = tauri::test::mock_app();
    let app = mock.handle().clone();
    // IMPORTANTE: usar a MESMA instância gerenciada tanto para lançar quanto
    // para o callback de saída, senão o mapa de "jogos rodando" (em memória)
    // não bate e o playtime não é somado.
    app.manage(Launcher::new().expect("launcher"));
    let launcher = app.state::<Launcher>();

    let instance = test_instance(&launcher).await;
    launch::install(&app, &launcher, &instance.id)
        .await
        .expect("instalar");

    let before = instances::load_instance(&launcher, &instance.id)
        .expect("carregar")
        .playtime_seconds;

    let pid = launch::launch(&app, &launcher, &instance.id, AuthSession::offline("TesteBot"))
        .await
        .expect("lançar");
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F", "/T"])
        .output();
    // Aguarda o callback de saída processar e salvar o playtime
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let after = instances::load_instance(&launcher, &instance.id)
        .expect("carregar 2")
        .playtime_seconds;
    println!("[teste] playtime {before}s -> {after}s");
    assert!(
        after >= before + 5,
        "playtime não foi contabilizado: {before} -> {after}"
    );
    println!("[teste] SUCESSO: playtime somou {}s", after - before);
}

/// Importa a skin de um jogador conhecido (Notch) pela API oficial da Mojang
/// e confere que virou uma skin na galeria.
#[tokio::test(flavor = "multi_thread")]
async fn importa_skin_por_nick() {
    use tauri::Manager;
    let mock = tauri::test::mock_app();
    let app = mock.handle().clone();
    app.manage(Launcher::new().expect("launcher"));
    let launcher = app.state::<Launcher>();

    let skin = skins::import_skin_from_player(launcher.clone(), "Notch".to_string())
        .await
        .expect("importar skin do Notch");
    println!("[teste] skin importada: {} ({})", skin.name, skin.variant);
    assert_eq!(skin.name, "Notch");

    let saved = skins::list_saved_skins(launcher).expect("listar skins");
    assert!(
        saved.iter().any(|s| s.skin.id == skin.id && !s.png_base64.is_empty()),
        "skin não apareceu na galeria com PNG"
    );
    println!("[teste] SUCESSO: skin do Notch na galeria");
}

/// Instala o Mod Menu na instância de teste e confere que as dependências
/// obrigatórias (Fabric API + Placeholder API) vieram junto.
#[tokio::test(flavor = "multi_thread")]
async fn instala_mod_com_dependencias() {
    let mock = tauri::test::mock_app();
    let app = mock.handle().clone();
    let launcher = Launcher::new().expect("launcher");

    let instance = test_instance(&launcher).await;
    let installed = content::install_content_inner(
        &app,
        &launcher,
        instance.id.clone(),
        "modmenu".into(),
        None,
    )
    .await
    .expect("instalar modmenu");
    println!("[teste] instalados: {installed:?}");
    assert!(
        installed.len() >= 3,
        "esperava modmenu + 2 dependências, veio: {installed:?}"
    );

    let mods_dir = launcher.instances_dir().join(&instance.id).join("mods");
    let mods: Vec<String> = std::fs::read_dir(&mods_dir)
        .expect("pasta mods")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .collect();
    assert!(
        mods.iter().any(|m| m.contains("modmenu")),
        "modmenu não está em mods/: {mods:?}"
    );
    assert!(
        mods.iter().any(|m| m.contains("fabric-api") || m.contains("fabric_api")),
        "dependência fabric-api não foi resolvida: {mods:?}"
    );
    println!("[teste] SUCESSO: modmenu + dependências instalados em mods/");
}
