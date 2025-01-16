using System;
using System.Threading.Tasks;
namespace Program
{
    class Program
    {
        static async Task Main(string[] args)
        {
            string serverAddress = "127.0.0.2"; 
            int commandPort = 7878;             
            string fileName = "new_file.txt";
            string fileName2 = "ceva.txt";   
            string savePath = "./downloads";    

            
            System.IO.Directory.CreateDirectory(savePath);

            using var ftpClient = new FileTransferClient.FiletransferKit.FileTransferClient(serverAddress, commandPort);

            try
            {
                await RunClients();
                // await ftpClient.GetFileAsync(fileName, savePath);
                // await ftpClient.GetFileAsync(fileName2, savePath);

                // await ftpClient.DeleteFileAsync(fileName);

                // var list = await ftpClient.ListFilesAsync();
                // foreach (var name in list)
                // {
                //     Console.WriteLine(name);
                // }

                // var list = await ftpClient.ListOwnedFilesAsync();
                // foreach (var name in list)
                // {
                //     Console.WriteLine(name);
                // }

                // await ftpClient.CreateFileAsync("new_file.txt","./downloads/ceva.txt");

                // await ftpClient.UpdateFileAsync("altceva.txt","./downloads/ceva.txt");

            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error: {ex.Message}");
            }
        }

        static async Task RunClients(int num_clients = 20)
        {
            string serverAddressPrefix = "127.0.0.";
            int commandPort = 7878;
            string fileName = "new_file.txt";
            string savePath = "./downloads";

            for (int i = 1; i <= num_clients; i++)
            {
                string serverAddress = serverAddressPrefix + i;
                Console.WriteLine($"{serverAddress}:{commandPort}");
                using var ftpClient = new FileTransferClient.FiletransferKit.FileTransferClient(serverAddress, commandPort);
                await ftpClient.GetFileAsync(fileName, savePath);
            }
            
            Thread.Sleep(1000);
            
        }
    }
}