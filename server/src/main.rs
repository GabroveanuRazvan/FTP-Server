use utils::server_utils::file_transfer_server::FileTransferServerBuilder;
use utils::server_utils::server_config::ServerConfig;

const DEFAULT_CONFIG_PATH: &str = "./config.json";

fn main() -> std::io::Result<()> {


    let server = FileTransferServerBuilder::new()
        .command_server_address(ServerConfig::get_command_address())
        .data_directory(ServerConfig::get_data_dir_path())
        .activate_ban_list()
        .serialized_lists_directory(ServerConfig::get_serialized_lists_path())
        .load_lists(ServerConfig::get_white_list_file_name().as_str(),ServerConfig::get_ban_list_file_name().as_str())
        .init_port_allocator(ServerConfig::get_first_port(),ServerConfig::get_last_port())
        .build();

    server.start()?;

    Ok(())

}
