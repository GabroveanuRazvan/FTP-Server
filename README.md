## Building and running the server

The server is run and built using docker compose:
```bash
  docker compose up -d --build
```

## Accessing server configurations via terminal

The server received inputs from its administrators. In order to access the command line of the server we must attach to it.

```bash
    docker attach file-transfer-server
```

### Build details
The app is compiled in a first phase using a Rust image.
After that the executable is moved into an Ubuntu image and ran there under an user with minimum privileges.
The created container will run on the [host network](https://docs.docker.com/engine/network/drivers/host/).
A volume is created that mounts the [server data](server_data) into the container.
The **CONFIG_PATH** environment variable is used to point to the [configuration file](/server_data/config.json) location.
As the server receives input command **stdin** and **tty** are set.