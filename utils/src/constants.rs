use std::net::{Ipv4Addr, SocketAddrV4};

/// File codes

pub const FILE_NOT_FOUND: &str = "File not found\n";
pub const DELETE_SUCCESSFUL: &str = "Deleted file successfully\n";
pub const DELETE_FAILED: &str = "Failed to delete file\n";
pub const ALREADY_EXISTS: &str = "File already exists\n";
pub const READY_TO_RECEIVE: &str = "File ready to receive\n";
pub const QUIT_MESSAGE: &str = "Bye!\n";
pub const UNRECOGNIZED_MESSAGE: &str = "Unrecognized command. Use HELP command for info.\n";

/// Miscellaneous

pub const EPHEMERAL_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
pub const FILE_TYPE_DIRECTORY: &str = "Directory";
pub const FILE_TYPE_FILE: &str = "File";
pub const FILE_TYPE_OTHER: &str = "Other";
pub const EMPTY: &str = "EMPTY";
pub const WRONG_INPUT: &str = "Wrong input!";
pub const BAN_LIST_DESC: &str = "Banned ips:";
pub const WHITE_LIST_DESC: &str = "Allowed ips:";
pub const CONFIG_LOAD_ERROR: &str = "Failed to load config file";

/// Verbs

pub const VERBS: [&str;8] = [GET,DELETE,LIST,CREATE,UPDATE,QUIT,HELP,LIST_OWNED];
pub const GET: &str = "GET";
pub const DELETE: &str = "DELETE";
pub const LIST: &str = "LIST";
pub const LIST_OWNED: &str = "LIST_OWNED";
pub const CREATE: &str = "CREATE";
pub const UPDATE: &str = "UPDATE";
pub const QUIT: &str = "QUIT";
pub const HELP: &str = "HELP";

/// Verb descriptions
pub const VERB_DESCRIPTIONS: [&str;7] = [GET_DESC,DELETE_DESC,LIST_DESC,CREATE_DESC,UPDATE_DESC,QUIT_DESC,LIST_OWNED_DESC];
pub const GET_DESC: &str = "Usage: GET <filename>";
pub const DELETE_DESC: &str = "Usage: DELETE <filename>";
pub const LIST_DESC: &str = "Usage: LIST";
pub const LIST_OWNED_DESC: &str = "Usage: LIST_OWNED";
pub const CREATE_DESC: &str = "Usage: CREATE <filename>";
pub const UPDATE_DESC: &str = "Usage: UPDATE <filename>";
pub const QUIT_DESC: &str = "Usage: QUIT";

/// Server input commands

pub const INPUTS: [&str;7] = [SHUTDOWN,ADD_IP,REMOVE_IP,LIST_IP,HELP,SWITCH,SHOW_CONFIG];
pub const SHUTDOWN: &str = "SHUTDOWN";
pub const ADD_IP: &str = "ADD";
pub const REMOVE_IP: &str = "REMOVE";
pub const LIST_IP: &str = "LIST";
pub const SWITCH: &str = "SWITCH";
pub const SHOW_CONFIG: &str = "SHOW_CONFIG";

/// Server input descriptions

pub const INPUT_DESCRIPTIONS: [&str;6] = [SHUTDOWN_DESC,ADD_IP_DESC,REMOVE_IP_DESC,LIST_IP_DESC,SWITCH_DESC,SHOW_CONFIG_DESC];
pub const SHUTDOWN_DESC: &str = "Usage: SHUTDOWN --- Shuts down the server and all active connections.";
pub const ADD_IP_DESC: &str = "Usage: ADD <ipv4 address> --- Adds a new IP to the white/ban list";
pub const REMOVE_IP_DESC: &str = "Usage: REMOVE<ipv4 address> --- Removes an IP from the white/ban list";
pub const LIST_IP_DESC: &str = "Usage: LIST --- Lists the white/ban list";
pub const SWITCH_DESC: &str = "Usage: SWITCH --- Switches from the current list to the opposite";

pub const SHOW_CONFIG_DESC: &str = "Usage: SHOW_CONFIG --- Shows current server configuration";

/// Server environment variables

pub const CONFIG_PATH_ENV: &str = "CONFIG_PATH";

/// Memory representation
pub const BYTE: usize = 1;
pub const KILOBYTE: usize = 1024 * BYTE;
pub const MEGABYTE: usize = 1024 * KILOBYTE ;

#[cfg(test)]

mod tests{
    use super::*;

    #[test]
    pub fn test_memory_representation_1(){
        assert_eq!(34 * BYTE, 34)
    }

    #[test]
    pub fn test_memory_representation_2(){
        assert_eq!(128 * KILOBYTE, 128 * 1024 * BYTE)
    }

    #[test]
    pub fn test_memory_representation_3(){
        assert_eq!(1000 * MEGABYTE, 1000 * 1024 * KILOBYTE * BYTE)
    }
}