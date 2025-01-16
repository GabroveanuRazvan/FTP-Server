use std::io;
use std::net::{Shutdown, SocketAddrV4, TcpStream};
use io::Result;
use std::io::{BufRead, BufReader, Read, Write};
use crate::constants::{KILOBYTE, CREATE, UPDATE, READY_TO_RECEIVE, EMPTY,QUIT};

const BUFFER_SIZE: usize = 4 * KILOBYTE;

pub struct FileTransferClient {
    client_address: SocketAddrV4,
}

/// Command line client to test the basic functionality of the server.
///
impl FileTransferClient {
    pub fn new(client_address: SocketAddrV4) -> Self {

        Self{
            client_address,
        }
    }

    /// Starts the client by waiting for inputs from stdin line by line.
    pub fn start(self) -> Result<()>{

        // Create the command connection
        let mut command_stream = TcpStream::connect(self.client_address)?;

        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        let reader = BufReader::new(io::stdin());
        println!("Ready to receive commands!");

        // Read line by line and treat each case
        'reader_loop: for line in reader.lines(){

            let line = format!("{}\n",line?);

            let parts: Vec<&str> = line.split_whitespace().collect();

            // write the request
            command_stream.write(line.as_bytes())?;

            // wait and read the data connection port
            command_stream.read(&mut buffer)?;
            let port = u16::from_be_bytes([buffer[0], buffer[1]]);

            // connect to the data stream and read responses
            let data_stream = TcpStream::connect((self.client_address.ip().clone(),port))?;

            // edge case
            let verb = if parts.len() == 0{
                EMPTY
            }else{
                &parts[0].to_uppercase()
            };


            match verb{

                CREATE => Self::update_or_create(data_stream,buffer.as_mut_slice())?,
                UPDATE => Self::update_or_create(data_stream,buffer.as_mut_slice())?,
                QUIT => {
                    Self::default(data_stream,buffer.as_mut_slice())?;
                    command_stream.shutdown(Shutdown::Both)?;
                    break 'reader_loop;
                }
                _ => Self::default(data_stream,buffer.as_mut_slice())?,

            }

        }
        println!("File transfer complete!");

        Ok(())

    }

    /// Treats any case besides correct CREATE or UPDATE requests by reading data from a data stream.
    pub fn default(mut data_stream: TcpStream,buffer: &mut [u8]) -> Result<()>{

        loop{

            match data_stream.read(buffer){

                Ok(0) => {
                    break;
                }

                Err(error) => return Err(error),

                Ok(bytes_read) =>{
                    let received_string = String::from_utf8_lossy(&buffer[0..bytes_read]);
                    println!("{received_string}");
                    // io::stdout().flush()?;
                }

            }

        }

        Ok(())
    }

    /// Treats UPDATE or CREATE requests by sending through the data stream the contents of the file to be created or updated.
    ///
    fn update_or_create(mut data_stream: TcpStream, buffer: &mut [u8]) -> Result<()> {

        let bytes_received = data_stream.read(buffer)?;
        let received_string = String::from_utf8_lossy(&buffer[0..bytes_received]);
        println!("{received_string}");

        if !received_string.starts_with(READY_TO_RECEIVE){

            return Ok(())
        }

        // println!("{received_string}");
        let reader = BufReader::new(io::stdin());

        for line in reader.lines(){

            let line = line?;

            if line.is_empty(){
                data_stream.shutdown(Shutdown::Both)?;
                break;
            }

            data_stream.write(line.as_bytes())?;
            data_stream.flush()?;


        }

        // Back to the command prompt
        println!("Ready to receive commands!");
        Ok(())
    }
}