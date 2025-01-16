using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Runtime.InteropServices.JavaScript;
using System.Text;
using System.Threading.Tasks;

namespace FileTransferClient.FiletransferKit
{
    public class FileTransferClient : IDisposable
    {
        private readonly string _serverAddress;
        private readonly int _commandPort;
        private TcpClient _commandClient;
        private NetworkStream _commandStream;
        private StreamReader _commandReader;
        private StreamWriter _commandWriter;

        /// <summary>
        /// Initializes the client with the server address.
        /// </summary>
        /// <param name="serverAddress">IPv4 string of the host.</param>
        /// <param name="commandPort">Port number of the host</param>
        public FileTransferClient(string serverAddress, int commandPort)
        {
            _serverAddress = serverAddress;
            _commandPort = commandPort;
        }

        /// <summary>
        /// Connects to the server obtaining the data stream, reader and writer.
        /// Must be called before sending any request as the server closes the connection everytime.
        /// </summary>
        private void Connect()
        {
            _commandClient = new TcpClient();
            _commandClient.Connect(_serverAddress, _commandPort);
            _commandStream = _commandClient.GetStream();
            _commandReader = new StreamReader(_commandStream, Encoding.ASCII);
            _commandWriter = new StreamWriter(_commandStream) { AutoFlush = true };
        }

        /// <summary>
        /// Builds a GET request and send it to the server.
        /// </summary>
        /// <param name="fileName">Name of the file to be fetched.</param>
        /// <param name="savePath">Path of the folder where the file will be stored.</param>
        /// <exception cref="FileNotFoundException">Exception thrown in case the file does not exist on the server.</exception>
        public async Task GetFileAsync(string fileName, string savePath)
        {   
            // Connect to the server
            Connect();
            
            // Build the GET request and send it to the server
            string request = $"{ServerVerbs.Get} {fileName}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader
            await using var dataStream = await GetDataStreamAsync();
            using var dataReader = new StreamReader(dataStream, Encoding.ASCII);
            
            // Read the first line and check if the requested file exists otherwise prepare the file to be stored
            string line;
            if ((line = await dataReader.ReadLineAsync()) != null && line.Equals(ServerResponses.FileNotFound))
                throw new FileNotFoundException($"File {fileName} not found.");
            
            // Build the path to store the file create the file and build its writer
            string fullPath = Path.Combine(savePath, fileName);
            await using var fileStream = new FileStream(fullPath, FileMode.Create, FileAccess.Write);
            await using var writer = new StreamWriter(fileStream, Encoding.ASCII);
                    
            // Store the previous first line
            await writer.WriteLineAsync(line);
            
            // Read in a loop until the server closes the data connection
            while ((line = await dataReader.ReadLineAsync()) != null)
            {
                await writer.WriteLineAsync(line);
            }
            
        }
        
        /// <summary>
        /// Sends a DELETE request to the server.
        /// </summary>
        /// <param name="fileName">Name of the file to be deleted.</param>
        /// <exception cref="FileNotFoundException">Exception thrown in case the file does not exist on the server.</exception>
        public async Task DeleteFileAsync(string fileName)
        {
            // Connect to the server
            Connect();
            
            // Build the DELETE request and send it to the server
            string request = $"{ServerVerbs.Delete} {fileName}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader
            await using var dataStream = await GetDataStreamAsync();
            using var dataReader = new StreamReader(dataStream, Encoding.ASCII);
            
            // Read the response and check if the request was successfull
            string response = await dataReader.ReadLineAsync();
            
            if(response != null && response.Equals(ServerResponses.FileNotFound))
                throw new FileNotFoundException($"File {fileName} not found.");
            
        }
        
        /// <summary>
        /// Sends a LIST request to the server.
        /// </summary>
        /// <returns> A list of strings containing all the files that can be fetched from the server.</returns>
        public async Task<List<string>> ListFilesAsync()
        {
            // Connect to the server
            Connect();
            
            // Build the LIST request and send it to the server
            string request = $"{ServerVerbs.List}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader
            await using var dataStream = await GetDataStreamAsync();
            using var dataReader = new StreamReader(dataStream, Encoding.ASCII);
            
            // Create the list where the file names will be stored
            List<string> file_names = new List<string>();
            string current_file_name;
            
            // Read a filename until the server closes the data connection
            while((current_file_name = await dataReader.ReadLineAsync()) != null)
                file_names.Add(current_file_name);
            
            return file_names;

        }
        
        /// <summary>
        /// Sends a LIST_OWNED request to the server.
        /// </summary>
        /// <returns> A list of strings containing all the owned files of this client.</returns>
        public async Task<List<string>> ListOwnedFilesAsync()
        {
            // Connect to the server
            Connect();
            
            // Build the LIST_OWNED request and send it to the server
            string request = $"{ServerVerbs.ListOwned}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader
            await using var dataStream = await GetDataStreamAsync();
            using var dataReader = new StreamReader(dataStream, Encoding.ASCII);
            
            // Create the list where the file names will be stored
            List<string> file_names = new List<string>();
            string current_file_name;
            
            // Read a filename until the server closes the data connection
            while((current_file_name = await dataReader.ReadLineAsync()) != null)
                file_names.Add(current_file_name);
            
            return file_names;
        }
        
        /// <summary>
        /// Creates a new file and sends its contents on the server.
        /// </summary>
        /// <param name="fileName">Name of the file on the server.</param>
        /// <param name="filePath">Path of the local file to be uploaded.</param>
        /// <exception cref="IOException">Exception thrown in case the file name already exists on the server.</exception>
        public async Task CreateFileAsync(string fileName, string filePath)
        {
            // Connect to the server
            Connect();
            
            // Build the CREATE request and sent it to the server
            string request = $"{ServerVerbs.Create} {fileName}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader and writer
            await using var dataStream = await GetDataStreamAsync();
            using var dataWriter = new StreamWriter(dataStream, Encoding.ASCII);
            using var dataReader= new StreamReader(dataStream, Encoding.ASCII);
            
            // Read the status code and treat each case
            string status_code = await dataReader.ReadLineAsync();
            
            if (status_code != null && status_code.Equals(ServerResponses.AlreadyExists))
                throw new IOException($"File {fileName} already exists.");

            if (filePath != null && !status_code.Equals(ServerResponses.ReadyToReceive))
                throw new IOException("Unknown status code.");
            
            // Build the stream reader of the file to store on the server
            await using var fileStream = new FileStream(filePath, FileMode.Open, FileAccess.Read);
            using var reader = new StreamReader(fileStream, Encoding.ASCII);

            const int bufferSize = 8192;
            char[] buffer = new char[bufferSize];
            int bytesRead;
            
            // Read chunks of the file and send them to the server through the data connection
            while ((bytesRead = await reader.ReadAsync(buffer, 0, bufferSize)) > 0)
            {
                
                await dataWriter.WriteAsync(buffer, 0, bytesRead);
                
            }
            
            // Ensure that the writer has sent every chunk
            await dataWriter.FlushAsync();
            dataStream.Close();
            
        }
        
        /// <summary>
        /// Updates an existing file on the server by truncating it and sending new data.
        /// </summary>
        /// <param name="fileName">Name of the file on the server.</param>
        /// <param name="filePath">Path of the local file to be uploaded.</param>
        /// <exception cref="FileNotFoundException">Exception thrown in case the file does not exist on the server.</exception>
        /// <exception cref="IOException">Exception thrown in case the status code is not recognized; should mark a server error.</exception>
        public async Task UpdateFileAsync(string fileName, string filePath)
        {
            // Connect to the server
            Connect();
            
            // Build the UPDATE request and sent it to the server
            string request = $"{ServerVerbs.Update} {fileName}\n";
            await _commandWriter.WriteLineAsync(request);
            
            // Get the data stream and build its reader and writer
            await using var dataStream = await GetDataStreamAsync();
            using var dataWriter = new StreamWriter(dataStream, Encoding.ASCII);
            using var dataReader= new StreamReader(dataStream, Encoding.ASCII);
            
            // Read the status code and treat each case
            string status_code = await dataReader.ReadLineAsync();
            
            if (status_code != null && status_code.Equals(ServerResponses.FileNotFound))
                throw new FileNotFoundException($"File {fileName} does not exist.");

            if (filePath != null && !status_code.Equals(ServerResponses.ReadyToReceive))
                throw new IOException("Unknown status code.");
            
            // Build the stream reader of the file to store on the server
            await using var fileStream = new FileStream(filePath, FileMode.Open, FileAccess.Read);
            using var reader = new StreamReader(fileStream, Encoding.ASCII);

            const int bufferSize = 8192;
            char[] buffer = new char[bufferSize];
            int bytesRead;
            
            // Read chunks of the file and send them to the server through the data connection
            while ((bytesRead = await reader.ReadAsync(buffer, 0, bufferSize)) > 0)
            {
                
                await dataWriter.WriteAsync(buffer, 0, bytesRead);
                
            }
            
            // Ensure that the writer has sent every chunk
            await dataWriter.FlushAsync();
            dataStream.Close();
        }
        
        /// <summary>
        /// Reads the port number from the command connection and then creates the data connection.
        /// </summary>
        /// <returns>NetworkStream of the data connection.</returns>
        private async Task<NetworkStream> GetDataStreamAsync()
        {
            // Read the port through the command connection
            ushort dataPort = await ReadPortAsync();
            
            // Create the data client and connect to it
            var dataClient = new TcpClient();
            await dataClient.ConnectAsync(_serverAddress, dataPort);
            
            // Return the stream
            return dataClient.GetStream();
        }
        
        /// <summary>
        /// Attempts to read 2 bytes from the command stream to convert them to the data port number.
        /// </summary>
        /// <returns>The port to be used to connect to the data connection.</returns>
        /// <exception cref="Exception">Exception thrown in case the bytes could not be read.</exception>
        private async Task<ushort> ReadPortAsync()
        {   
            // Read 2 bytes from the connection and convert them to a port value
            byte[] buffer = new byte[2];
            int bytesRead = await _commandStream.ReadAsync(buffer, 0, 2);
          
            if (bytesRead != 2)
            {
                throw new Exception("Port receive error.");
            }

            // Convert to little endian 
            if (BitConverter.IsLittleEndian)
            {
                Array.Reverse(buffer);
            }

            ushort port = BitConverter.ToUInt16(buffer, 0);
            
            return port;
        }

        public void Dispose()
        {
            _commandReader?.Dispose();
            _commandWriter?.Dispose();
            _commandStream?.Dispose();
            _commandClient?.Close();
            
        }
    }
}
