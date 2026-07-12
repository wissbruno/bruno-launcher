pub mod content;
pub mod error;
pub mod instances;
pub mod minecraft;
pub mod msauth;
pub mod settings;
pub mod state;

use state::Launcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launcher = Launcher::new().expect("falha ao iniciar diretórios do launcher");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(launcher)
        .invoke_handler(tauri::generate_handler![
            // Instâncias
            instances::list_instances,
            instances::create_instance,
            instances::delete_instance,
            instances::rename_instance,
            instances::open_instance_folder,
            instances::list_instance_content,
            instances::remove_instance_content,
            // Minecraft
            minecraft::launch::get_game_versions,
            minecraft::launch::get_loader_versions,
            minecraft::launch::prepare_instance,
            minecraft::launch::launch_instance,
            minecraft::launch::kill_instance,
            minecraft::launch::get_running,
            // Conteúdo Modrinth
            content::install_content,
            content::install_modpack,
            // Configurações
            settings::get_settings,
            settings::set_settings,
            // Conta Microsoft
            msauth::msa_begin,
            msauth::msa_poll,
            msauth::get_accounts,
            msauth::set_active_account,
            msauth::remove_account,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o app");
}
