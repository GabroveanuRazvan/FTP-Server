use std::env;
use std::net::SocketAddrV4;
use std::path::{PathBuf};
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use crate::constants;
use crate::constants::CONFIG_PATH_ENV;
use crate::serialization::load;

const DEFAULT_CONFIG_PATH: &str = "./config.json";

/// Once initialized only structure for the server configurations.
///
pub static CONFIG_DATA: OnceLock<ServerConfig> = OnceLock::new();

/// Structure used to store all server configurations.
///
#[derive(Debug,Deserialize,Serialize)]
pub struct ServerConfig {
    pub command_address: SocketAddrV4,
    pub data_dir_path: PathBuf,
    pub  serialized_lists_path: PathBuf,
    pub white_list_file_name: String,
    pub ban_list_file_name: String,
    pub server_num_threads: usize,
    pub buffer_size: usize,
    pub first_port: u16,
    pub last_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self{
            command_address: constants::EPHEMERAL_ADDRESS,
            data_dir_path: PathBuf::default(),
            serialized_lists_path: PathBuf::default(),
            white_list_file_name: String::default(),
            ban_list_file_name: String::default(),
            server_num_threads: 0,
            buffer_size: 0,
            first_port: 1,
            last_port: 2,
        }
    }
}

//
impl ServerConfig {

    /// Initializes the config data on the first call and returns it.
    /// The config file path is set using the CONFIG_PATH environment variable.
    /// If the environment variable is not set a default path is used.
    pub fn get_config() -> &'static ServerConfig {

        CONFIG_DATA.get_or_init(||{



            match env::var(CONFIG_PATH_ENV){
                Ok(config_path) => load::<ServerConfig,String>(config_path.clone()).expect(&format!("{} {}",constants::CONFIG_LOAD_ERROR,config_path)),
                Err(_) => load::<ServerConfig,&str>(DEFAULT_CONFIG_PATH).expect(&format!("{} {}",constants::CONFIG_LOAD_ERROR,DEFAULT_CONFIG_PATH)),
            }

        })
    }

    pub fn get_command_address() -> SocketAddrV4 {
        Self::get_config().command_address.clone()
    }
    pub fn get_data_dir_path() -> PathBuf {
        Self::get_config().data_dir_path.clone()
    }
    pub fn get_serialized_lists_path() -> PathBuf { Self::get_config().serialized_lists_path.clone() }
    pub fn get_white_list_file_name() -> String {
        Self::get_config().white_list_file_name.clone()
    }
    pub fn get_ban_list_file_name() -> String {
        Self::get_config().ban_list_file_name.clone()
    }
    pub fn get_server_num_threads() -> usize {
        Self::get_config().server_num_threads
    }
    pub fn get_buffer_size() -> usize {
        Self::get_config().buffer_size
    }
    pub fn get_first_port() -> u16 {Self::get_config().first_port}
    pub fn get_last_port() -> u16 {Self::get_config().last_port}
}


#[cfg(test)]

mod tests {
    use crate::constants::CONFIG_PATH_ENV;
    use super::*;

    #[test]
    pub fn test_server_config_1(){

        env::set_var(CONFIG_PATH_ENV, "./tests/config.json");
        let server_config = ServerConfig::get_config();

        assert_eq!(ServerConfig::get_buffer_size(),8192);
        assert_eq!(ServerConfig::get_server_num_threads(),10);
        assert_eq!(ServerConfig::get_data_dir_path(),PathBuf::from("./data"));

    }
}