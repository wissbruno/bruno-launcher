pub mod content;
pub mod error;
pub mod instances;
pub mod minecraft;
pub mod msauth;
pub mod settings;
pub mod skins;
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
            instances::duplicate_instance,
            instances::set_instance_icon,
            instances::get_instance_icon,
            instances::set_instance_pinned,
            instances::set_instance_details,
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
            content::check_mod_updates,
            content::apply_mod_updates,
            content::export_modpack,
            // Configurações
            settings::get_settings,
            settings::set_settings,
            // Conta Microsoft
            msauth::msa_begin,
            msauth::msa_poll,
            msauth::get_accounts,
            msauth::set_active_account,
            msauth::remove_account,
            msauth::get_skin,
            msauth::upload_skin,
            // Galeria de skins
            skins::list_saved_skins,
            skins::add_saved_skin,
            skins::import_skin_from_player,
            skins::import_skin_from_url,
            skins::delete_saved_skin,
            skins::set_favorite_skin,
            skins::apply_saved_skin,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o app");
}
