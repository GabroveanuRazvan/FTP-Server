use std::{fs, io, mem, thread};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use io::Result;
use std::collections::HashSet;
use std::fs::{create_dir_all, remove_file, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::SocketAddr::V6;
use std::net::SocketAddr::V4;
use std::path::{PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use crate::constants::*;
use crate::directory_tree::DirectoryTree;
use crate::server_utils::file_transfer_server::ActiveList::{BanList, WhiteList};
use crate::mapped_file::MappedFile;
use crate::serialization::{format_ipv4, load, save};
use crate::server_utils::port_allocator::PortAllocator;
use crate::server_utils::server_config::ServerConfig;
use crate::thread_pool::ThreadPool;

/// Basic file transfer server.
///

type ProtectedSet<T> = Arc<RwLock<HashSet<T>>>;
type ProtectedType<T> = Arc<RwLock<T>>;

static PORT_ALLOCATOR: OnceLock<Arc<PortAllocator>> = OnceLock::new();

#[derive(Debug)]
pub struct FileTransferServer {
    command_server_address: SocketAddrV4,
    data_directory: PathBuf,
    serialized_lists_directory: Option<PathBuf>,

    active_list: ProtectedType<ActiveList>,
    white_list: ProtectedSet<Ipv4Addr>,
    ban_list: ProtectedSet<Ipv4Addr>,
    white_list_name: String,
    ban_list_name: String,
}

impl FileTransferServer {

    /// Binds the command server to its address, creates the storage directory and sets the current directory.
    /// Handles each new client in a loop using a thread pool.
    ///
    pub fn start(mut self) -> Result<()>{

        // Bind and set non-blocking to true
        let command_server = TcpListener::bind(self.command_server_address)?;
        command_server.set_nonblocking(true)?;

        println!("Server started on {}", self.command_server_address);

        // Init the shutdown signal, create the data directory and set the current directory to iy
        let shutdown_signal = Arc::new(AtomicBool::new(false));


        // Start the thread pool and the input thread
        let mut thread_pool = ThreadPool::new(ServerConfig::get_server_num_threads());


        let input_thread_handle = Self::input_thread(Arc::clone(&shutdown_signal),
                                                                         Arc::clone(&self.white_list),
                                                                         Arc::clone(&self.ban_list),
                                                                         Arc::clone(&self.active_list));

        while !shutdown_signal.load(Ordering::Relaxed) {

            // Non-blocking accept in order to handle the shutdown signal
            match command_server.accept(){

                Ok((stream,address)) =>{
                    let signal_clone = Arc::clone(&shutdown_signal);
                    let data_dir_clone = self.data_directory.clone();

                    let client_ip = match Self::ipv4_from_sockaddr(address){
                        Some(addr) => addr,
                        None => continue,
                    };

                    println!("{address:?}");
                    // Handle the client

                    match self.active_list.read().unwrap().clone(){

                        // Case when the ban list is selected and the client is banned
                        BanList if self.ban_list.read().unwrap().contains(&client_ip) => {

                            stream.shutdown(Shutdown::Both)?;
                            continue;
                        },

                        // Case when the white list is selected and the client is not on the white list
                        WhiteList if !self.white_list.read().unwrap().contains(&client_ip) => {
                            stream.shutdown(Shutdown::Both)?;
                            continue;
                        },

                        // Case when the client has access
                        _ => thread_pool.execute(move || {
                                Self::handle_client_once(stream,Arc::clone(&signal_clone),data_dir_clone).unwrap();
                            }),

                    }


                }

                // If the accept call would wait for a new client just simulate the block
                // in order not to consume too many CPU resources
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }

                Err(e) => {
                    return Err(e);
                }

            }

        }

        // drop the thread pool and wait for the input pool to finish
        drop(thread_pool);
        input_thread_handle.join().unwrap()?;
        Ok(())
    }

    /// STDIN thread waiting to receive commands:
    /// SHUTDOWN - shutdowns the server by sending a signal
    /// ADD <IP> - Adds a new ip to the white/ban list
    /// REMOVE <IP> - Removes an ip from the white/ban list
    /// LIST - Lists the white/ban list ips
    /// SWITCH - Switches current ips list to the opposite one
    /// SWITCH - Switches current ips list to the opposite one
    /// HELP - Lists the commands
    fn input_thread(shutdown_signal: Arc<AtomicBool>,
                    white_list: ProtectedSet<Ipv4Addr>,
                    ban_list: ProtectedSet<Ipv4Addr>,
                    active_list: ProtectedType<ActiveList>) -> JoinHandle<Result<()>> {

        thread::spawn(move || {

            let mut reader = BufReader::new(io::stdin());


            let mut current_list: ProtectedSet<Ipv4Addr> = match active_list.read().unwrap().clone(){

                BanList => Arc::clone(&ban_list),
                WhiteList => Arc::clone(&white_list),
            };

            let mut current_list_desc = match active_list.read().unwrap().clone(){
                WhiteList => WHITE_LIST_DESC,
                BanList => BAN_LIST_DESC,
            };

            for line in reader.lines(){

                let line = line?.to_uppercase();
                let parts : Vec<&str>= line.split_whitespace().collect();

                // Edge case for empty string
                let verb = if parts.len() == 0{
                    EMPTY.to_string()
                }else{
                    parts[0].to_uppercase()
                };

                let verb = verb.as_str();

                // Extract the second argument or get a default value to it that will fail in parsings
                let second_argument = match parts.len(){
                    2 => parts[1].to_string(),
                    _ => EMPTY.to_string(),
                };

                match verb{

                    SHUTDOWN =>{
                        Self::shutdown_input(Arc::clone(&shutdown_signal));
                        break;
                    }

                    ADD_IP => Self::add_ip_input(second_argument, Arc::clone(&current_list)),
                    REMOVE_IP => Self::remove_ip_input(second_argument, Arc::clone(&current_list)),
                    LIST_IP => Self::list_ip_input(Arc::clone(&current_list), current_list_desc),

                    SWITCH => {

                        match active_list.read().unwrap().clone(){
                            WhiteList => {
                                current_list = Arc::clone(&ban_list);
                                current_list_desc = BAN_LIST_DESC;
                            },
                            BanList => {
                                current_list = Arc::clone(&white_list);
                                current_list_desc = WHITE_LIST_DESC;
                            },
                        }

                        active_list.write().unwrap().switch();
                        println!()
                    },

                    SHOW_CONFIG => Self::show_config_input(),

                    HELP => Self::help_input(),

                    _ => Self::unrecognized_input(),
                }

            }

            Ok(())
        })

    }

    /// Lists the ips of the selected list.
    ///
    fn list_ip_input(current_list: ProtectedSet<Ipv4Addr>, list_description: &str){

        println!("{}",list_description);
        for ip in current_list.read().unwrap().iter(){
            println!("{}", ip);
        }
        println!();
    }

    /// Parses a string into an ipv4 and adds it to the current list if the parsing was successful.
    ///
    fn add_ip_input(ip: String, current_list: ProtectedSet<Ipv4Addr>){

        let ip = ip.parse::<Ipv4Addr>();

        match ip{
            Err(_) => println!("{}",WRONG_INPUT),
            Ok(ip) => {
                current_list.write().unwrap().insert(ip);
            },
        }
        println!();

    }
    /// Parses a string into an ipv4 and removes it from the current list if the parsing was successful.
    ///
    fn remove_ip_input(ip: String, current_list: Arc<RwLock<HashSet<Ipv4Addr>>>) {

        let ip = ip.parse::<Ipv4Addr>();

        match ip{
            Err(_) => println!("{}",WRONG_INPUT),
            Ok(ip) => {
                current_list.write().unwrap().remove(&ip);
            },
        }
        println!();

    }


    /// Shuts down the server by setting the shutdown signal to true.
    ///
    fn shutdown_input(shutdown_signal: Arc<AtomicBool>) {
        println!("Shutting down");
        shutdown_signal.store(true, Ordering::Relaxed);
        println!();
    }

    /// Prints current server configuration.
    ///
    fn show_config_input(){
        println!("{:#?}",ServerConfig::get_config());
        println!()
    }

    /// Prints all the inputs and their usages.
    ///
    fn help_input(){

        for description in INPUT_DESCRIPTIONS{
            println!("{}", description);
        }
        println!();

    }

    /// Prints a warning message.
    ///
    fn unrecognized_input(){
        println!("Unrecognized input");
        println!();
    }

    /// Handles a single client request, then shuts down the connection.
    ///
    fn handle_client_once(mut stream: TcpStream,shutdown_signal: Arc<AtomicBool>, data_directory: PathBuf) -> Result<()>{

        let address = stream.local_addr()?;
        let client_ip = Self::ipv4_from_sockaddr(address).unwrap();
        let client_dir_path = data_directory.join(format_ipv4(client_ip));

        if !client_dir_path.exists(){
            fs::create_dir(&client_dir_path)?;
        }

        let invalid_path = data_directory.join(EMPTY);
        let data_dir_tree = DirectoryTree::new(data_directory)?;

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        stream.set_nonblocking(true)?;

        // Check in a loop if the shutdown signal is set then try to read a new line
        loop{

            // Check the shutdown signal
            if shutdown_signal.load(Ordering::Relaxed){
                stream.shutdown(Shutdown::Both)?;
                return Ok(());
            }

            match reader.read_line(&mut line){

                // If the read call would block just sleep for a small quantum of time
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }

                Err(e) => return Err(e),

                Ok(_) => {break;}

            };
        }


        let parts : Vec<&str>= line.split_whitespace().collect();

        // Edge case for empty string
        let verb = if parts.len() == 0{
            EMPTY.to_string()
        }else{
            parts[0].to_uppercase()
        };

        let verb = verb.as_str();

        let file_path = match parts.len(){
            2 => Some(parts[1]),
            _ => None,
        };

        match verb{

            GET => {
                let writer_stream = stream.try_clone()?;

                match file_path {
                    Some(file_path) =>{
                        let path = data_dir_tree.find_file(file_path)?.unwrap_or_else(||invalid_path.clone());
                        Self::get(path,writer_stream)?
                    },
                    None => Self::send_verb_details(GET,writer_stream)?
                };

            },

            DELETE => {
                let writer_stream = stream.try_clone()?;

                match file_path {
                    Some(file_path) =>{
                        let path = client_dir_path.join(file_path);
                        Self::delete(path,writer_stream)?
                    },
                    None => Self::send_verb_details(DELETE,writer_stream)?
                };
            },

            CREATE => {
                let writer_stream = stream.try_clone()?;

                match file_path {
                    Some(file_path) => {
                        let path = client_dir_path.join(file_path);
                        Self::create(data_dir_tree.clone(),path,writer_stream)?
                    },
                    None => Self::send_verb_details(CREATE,writer_stream)?
                };
            }

            UPDATE => {
                let writer_stream = stream.try_clone()?;

                match file_path {
                    Some(file_path) => {
                        let path = client_dir_path.join(file_path);
                        Self::update(path,writer_stream)?
                    },
                    None => Self::send_verb_details(UPDATE,writer_stream)?
                };
            }

            LIST => {
                let writer_stream = stream.try_clone()?;
                Self::list(data_dir_tree.clone(), writer_stream)?;
            }

            LIST_OWNED => {
                let writer_stream = stream.try_clone()?;
                let client_dir_tree = DirectoryTree::new(client_dir_path)?;
                Self::list(client_dir_tree,writer_stream)?;
            }

            QUIT => {
                let writer_stream = stream.try_clone()?;
                Self::quit(writer_stream)?;
                return Ok(());
            }

            HELP => {
                let writer_stream = stream.try_clone()?;
                Self::help(writer_stream)?
            }

            _ => {
                let writer_stream = stream.try_clone()?;
                Self::unrecognized(writer_stream)?
            },

        }

        // Shutdown the command connection if it has not been shut already
        // If it has already been shut this call will return an Err() which SHOULD NOT BE PROPAGATED
        stream.shutdown(Shutdown::Both);

        Ok(())

    }

    /// Creates a passive socket on an ephemeral port.
    /// Send the port number to the client via the writer stream.
    ///
    fn create_data_stream(mut writer_stream: TcpStream) -> Result<TcpStream>{

        let port = Self::get_port_allocator().alloc();
        let address = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

        // Create a new socket on an ephemeral port
        let data_server = TcpListener::bind(address)?;
        let data_port = data_server.local_addr()?.port();

        println!("{data_port}");
        // Send the port to the client through the command connection
        let mut port_bytes_message = Vec::new();
        port_bytes_message.extend_from_slice(&data_port.to_be_bytes());
        port_bytes_message.push('\n' as u8);

        writer_stream.write_all(port_bytes_message.as_slice())?;
        println!("Sent port");
        // Wait for the client to connect to the data connection
        let (data_stream,_address) = data_server.accept()?;
        println!("Accepted");
        Ok(data_stream)
    }

    /// Treat a get request.
    /// Create a memory mapped file to be sent in chunks through a data connection.
    ///
    fn get(file_path: PathBuf, writer_stream: TcpStream) -> Result<()> {

        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let mut data_port = data_stream.local_addr()?.port();

        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .truncate(false)
            .create(false)
            .open(&file_path);

        // Send the file if it exists otherwise send an error code
        match file{

            Ok(file) => {

                // map the current file
                let mmap = unsafe{Mmap::map(&file)?};

                // Send the file in chunks
                for chunk in mmap.chunks(ServerConfig::get_buffer_size()){
                    data_stream.write_all(chunk)?;
                }

            }

            Err(_error) => {
                data_stream.write_all(FILE_NOT_FOUND.as_bytes())?;
            }
        }

        // Shutdown the temporary data connection and free the port
        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);
        Ok(())

    }

    /// Deletes a file and sends a message to mark the status through a temporary connection.
    fn delete(file_path: PathBuf, writer_stream: TcpStream) -> Result<()> {

        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let mut data_port = data_stream.local_addr()?.port();

        let remove_result = remove_file(&file_path);

        match remove_result {
            Ok(_) => {
                data_stream.write_all(DELETE_SUCCESSFUL.as_bytes())?;
            }

            Err(_error) => {
                data_stream.write_all(FILE_NOT_FOUND.as_bytes())?;
            }
        }

        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);

        Ok(())
    }

    /// Attempts to create the given file. If it already exists, a message is sent through the data stream and the connection ends.
    /// If the file is a new one, transmits through the data stream a ready message, and reads chunks of the file in a loop
    /// until the data connection is ended.
    ///
    fn create(data_dir_tree: DirectoryTree<PathBuf>, file_path: PathBuf, writer_stream: TcpStream) -> Result<()>  {

        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let data_port = data_stream.local_addr()?.port();

        let file_name = file_path.file_name().unwrap().to_str().unwrap();

        // If there is another file named the same across the data directory signal it
        match data_dir_tree.find_file(file_name)?{
            None => (),
            Some(_) => {
                data_stream.write_all(ALREADY_EXISTS.as_bytes())?;
                data_stream.shutdown(Shutdown::Both)?;
                Self::get_port_allocator().dealloc(data_port);
                return Ok(())
            }
        }

        // Create the file if it does not exist, else send a message through the data stream and end the connection
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .append(true)
            .truncate(false)
            .open(&file_path);

        let file = match file{

            Err(error) => {

                return match error.kind() {
                    io::ErrorKind::AlreadyExists => {
                        data_stream.write_all(ALREADY_EXISTS.as_bytes())?;
                        data_stream.shutdown(Shutdown::Both)?;
                        Self::get_port_allocator().dealloc(data_port);
                        Ok(())
                    },

                    _ => Err(error)
                }

            }

            Ok(file) => {file}
        };

        // Write to announce that the file can be transferred
        data_stream.write_all(READY_TO_RECEIVE.as_bytes())?;

        // Map the file and append the received chunks through the data connection.
        let mut mapped_file = MappedFile::new(file)?;
        let mut receive_buffer = vec![0; ServerConfig::get_buffer_size()];

        loop{

            match data_stream.read(&mut receive_buffer){

                Ok(0) => break,

                Ok(bytes_received) => mapped_file.write_append(&receive_buffer[..bytes_received])?,

                Err(error) => {
                    return Err(error);
                }

            }

        }

        Self::get_port_allocator().dealloc(data_port);
        Ok(())
    }

    /// Attempts to open and truncate the given file. If it does not exist, a message is sent through the data stream and the connection ends.
    /// If the file is a new one, transmits through the data stream a ready message, and reads chunks of the file in a loop
    /// until the data connection is ended.
    ///
    fn update(path: PathBuf, writer_stream: TcpStream) -> Result<()>  {

        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let data_port = data_stream.local_addr()?.port();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .append(false)
            .truncate(true)
            .create(false)
            .open(&path);


        let file = match file{

            Err(error) => {

                return match error.kind() {
                    io::ErrorKind::NotFound => {
                        data_stream.write_all(FILE_NOT_FOUND.as_bytes())?;
                        data_stream.shutdown(Shutdown::Both)?;
                        Self::get_port_allocator().dealloc(data_port);
                        Ok(())
                    },

                    _ => Err(error)
                }

            }

            Ok(file) => {file}
        };

        // Write to announce that the file can be transferred
        data_stream.write_all(READY_TO_RECEIVE.as_bytes())?;

        // Map the file and append the received chunks through the data connection.
        let mut mapped_file = MappedFile::new(file)?;
        let mut receive_buffer = vec![0; ServerConfig::get_buffer_size()];

        loop{

            match data_stream.read(&mut receive_buffer){

                Ok(0) => break,

                Ok(bytes_received) => mapped_file.write_append(&receive_buffer[..bytes_received])?,

                Err(error) => {
                    return Err(error);
                }

            }

        }

        Self::get_port_allocator().dealloc(data_port);
        Ok(())
    }

    /// Send an end connection message through the data connection and shutdown the command connection.
    ///
    fn quit(mut writer_stream: TcpStream) -> Result<()> {

        let mut data_stream = Self::create_data_stream(writer_stream.try_clone().unwrap())?;
        let data_port = data_stream.local_addr()?.port();

        data_stream.write_all(QUIT_MESSAGE.as_bytes())?;
        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);

        writer_stream.shutdown(Shutdown::Both)?;

        Ok(())
    }


    /// Sends the verb details through a data stream.
    ///
    fn send_verb_details(verb: &str, mut writer_stream: TcpStream) -> Result<()> {

        let mut data_stream = Self::create_data_stream(writer_stream.try_clone().unwrap())?;
        let data_port = data_stream.local_addr()?.port();

        match verb{

            GET => data_stream.write_all(GET_DESC.as_bytes())?,
            DELETE => data_stream.write_all(DELETE_DESC.as_bytes())?,
            CREATE => data_stream.write_all(CREATE_DESC.as_bytes())?,
            UPDATE => data_stream.write_all(UPDATE_DESC.as_bytes())?,
            QUIT => data_stream.write_all(QUIT_DESC.as_bytes())?,
            _ => data_stream.write_all("How did you get here?".as_bytes())?,

        }

        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);

        Ok(())
    }

    /// Lists all files in the hierarchy of the current directory.
    ///
    fn list(data_dir_tree: DirectoryTree<PathBuf>,writer_stream: TcpStream) -> Result<()> {

        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let data_port = data_stream.local_addr()?.port();

        let file_paths = data_dir_tree.list_files_in_tree()?;

        for path in file_paths {

            let file_name = path.file_name().unwrap();
            let formatted_name = format!("{}\n",file_name.to_str().unwrap());
            data_stream.write_all(formatted_name.as_bytes())?;

        }

        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);

        Ok(())
    }

    /// The server sends the usages of each verb.
    ///
    fn help(writer_stream: TcpStream) -> Result<()> {
        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let data_port = data_stream.local_addr()?.port();

        for description in VERB_DESCRIPTIONS{
            let line = format!("{description}\n");
            data_stream.write_all(line.as_bytes())?;
        }

        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);
        Ok(())
    }

    /// Treats an unrecognized request.
    ///
    fn unrecognized(writer_stream: TcpStream) -> Result<()> {
        let mut data_stream = Self::create_data_stream(writer_stream)?;
        let data_port = data_stream.local_addr()?.port();

        data_stream.write_all(UNRECOGNIZED_MESSAGE.to_string().as_bytes())?;

        data_stream.shutdown(Shutdown::Both)?;
        Self::get_port_allocator().dealloc(data_port);
        Ok(())
    }

    /// Saves the current lists into the specified directory.
    /// Panics if the serialization fails.
    fn save_lists(&self){

        if let Some(directory) = &self.serialized_lists_directory {

            let white_list_path = PathBuf::from(directory).join(&self.white_list_name);
            let ban_list_path = PathBuf::from(directory).join(&self.ban_list_name);

            let white_list = self.white_list.read().unwrap().clone();
            let ban_list = self.ban_list.read().unwrap().clone();

            save(white_list,white_list_path).expect("Failed to save white list");
            save(ban_list,ban_list_path).expect("Failed to save ban list");

        }

    }

    /// Extracts the ipv4 of a std::net::SocketAddr if it exists.
    ///
    fn ipv4_from_sockaddr(address: SocketAddr) -> Option<Ipv4Addr>{

        match address{
            V4(addr) => Some(addr.ip().clone()),
            V6(_addr) => None
        }

    }

    /// Gets an owned reference count of the allocator.
    /// Will panic if the allocator was not initialized.
    fn get_port_allocator() -> Arc<PortAllocator>{
        Arc::clone(PORT_ALLOCATOR.get().unwrap())
    }

}

/// After drop the server will save its lists as json files.
///
impl Drop for FileTransferServer {
    fn drop(&mut self) {
        self.save_lists()
    }
}

/// Simple builder for a File Transfer Server
///
pub struct FileTransferServerBuilder{
    command_server_address: SocketAddrV4,
    data_directory: PathBuf,
    serialized_lists_directory: Option<PathBuf>,

    active_list: ActiveList,
    white_list: HashSet<Ipv4Addr>,
    ban_list: HashSet<Ipv4Addr>,
    white_list_name: String,
    ban_list_name: String,
}

impl FileTransferServerBuilder {

    pub fn new() -> Self{
        FileTransferServerBuilder{
            command_server_address: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8080),
            data_directory: PathBuf::from("./"),
            serialized_lists_directory: None,

            active_list: WhiteList,
            white_list: HashSet::new(),
            ban_list: HashSet::new(),
            white_list_name: String::new(),
            ban_list_name: String::new(),
        }
    }

    pub fn command_server_address(mut self, address: SocketAddrV4) -> Self {
        self.command_server_address = address;
        self
    }

    pub fn command_ipv4(mut self,ip: Ipv4Addr) -> Self{
        self.command_server_address.set_ip(ip);
        self
    }

    pub fn command_port(mut self, port: u16) -> Self{
        self.command_server_address.set_port(port);
        self
    }


    pub fn data_directory(mut self,dir: PathBuf) -> Self{

        create_dir_all(dir.clone()).expect("Failed to create data directory");
        self.data_directory = dir;
        self
    }

    pub fn activate_ban_list(mut self) -> Self{
        self.active_list = BanList;
        self
    }

    pub fn activate_white_list(mut self) -> Self{
        self.active_list = WhiteList;
        self
    }

    pub fn serialized_lists_directory(mut self,dir: PathBuf) -> Self{

        create_dir_all(dir.clone()).expect("Failed to create serialized lists directory");
        self.serialized_lists_directory = Some(dir);
        self
    }

    /// Loads the lists given as a parameter from the serialized lists directory if it exists.
    ///
    pub fn load_lists(mut self,white_list_name: &str,ban_list_name: &str)-> Self{

        self.white_list_name = white_list_name.to_string();
        self.ban_list_name = ban_list_name.to_string();

        if let Some(directory) = &self.serialized_lists_directory {

            let white_list_path = PathBuf::from(directory).join(white_list_name);
            let ban_list_path = PathBuf::from(directory).join(ban_list_name);

            self.white_list = load(white_list_path).unwrap_or_else(|_| HashSet::new());
            self.ban_list = load(ban_list_path).unwrap_or_else(|_| HashSet::new());
        }


        self
    }

    /// Constructs the singleton port allocator of this struct.
    pub fn init_port_allocator(self,first_port: u16, last_port: u16) -> Self{
        PORT_ALLOCATOR.get_or_init(|| Arc::new(PortAllocator::new(first_port, last_port)));
        self
    }

    pub fn build(self) -> FileTransferServer{

        FileTransferServer{
            command_server_address:self.command_server_address,
            data_directory: self.data_directory,
            serialized_lists_directory: self.serialized_lists_directory,

            active_list: Arc::new(RwLock::new(self.active_list)),
            white_list: Arc::new(RwLock::new(self.white_list)),
            ban_list: Arc::new(RwLock::new(self.ban_list)),
            white_list_name: self.white_list_name,
            ban_list_name: self.ban_list_name,
        }
    }

}

#[derive(Serialize,Deserialize,Debug,Clone)]
enum ActiveList{
    BanList,
    WhiteList,
}

impl ActiveList{
    pub fn is_ban_list(&self) -> bool {
        match &self{
            BanList => true,
            WhiteList => false,
        }
    }

    pub fn is_white_list(&self) -> bool {
        match &self{
            WhiteList => true,
            BanList => false,
        }
    }

    pub fn switch(&mut self){
        match &self{

            BanList => {mem::replace(self, WhiteList);},
            WhiteList => {mem::replace(self, BanList);},
        }
    }

}

#[cfg(test)]

mod tests{
    use super::*;

    #[test]
    pub fn test_active_list_1(){
        let active_list = BanList;

        assert!(active_list.is_ban_list());
    }

    #[test]
    pub fn test_active_list_2(){
        let active_list = WhiteList;

        assert!(active_list.is_white_list());
    }

    #[test]
    pub fn test_active_list_3(){
        let mut active_list = BanList;
        active_list.switch();

        match active_list{
            BanList => assert!(false),
            WhiteList => assert!(true),
        }
    }

    #[test]
    pub fn test_active_list_4(){
        let mut active_list = WhiteList;
        active_list.switch();

        match active_list{
            BanList => assert!(true),
            WhiteList => assert!(false),
        }
    }
}