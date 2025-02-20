use crossterm::style::{style, Color, Stylize};
use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    lock::Mutex,
    net::{SocketAddr, TcpListener, TcpStream},
    stream::StreamExt,
};
use std::{fmt::Display, result, sync::Arc};

const ADDRESS: &str = "127.0.0.1:3000";
const SAFE_MODE: bool = false;

type Result<T> = result::Result<T, ()>;

struct Sesitive<T>(T);

impl<T> Display for Sesitive<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Sesitive(data) = self;
        if SAFE_MODE {
            "[REDACTED]".fmt(f)
        } else {
            data.fmt(f)
        }
    }
}

#[derive(Debug, Clone)]
struct Client {
    addr: SocketAddr,
    stream: TcpStream,
}

impl Client {
    fn new(addr: SocketAddr, stream: TcpStream) -> Self {
        Client { addr, stream }
    }

    async fn broadcast(&self, conn_clients: Arc<Mutex<Vec<Client>>>, msg: &[u8]) {
        let curr_addr = self.addr;
        let mut clients = conn_clients.lock().await;

        for client in clients.iter_mut() {
            if client.addr != curr_addr {
                let _bytes_written = client.stream.write(msg).await.unwrap_or_default();
            }
        }
    }

    async fn handle_connection(&mut self, conn_clients: Arc<Mutex<Vec<Client>>>) -> Result<()> {
        let conn_msg = format!("{} @ {}", Msg::ClientConnected, self.addr);
        println!("{}", style(&conn_msg).with(Color::Green));

        let clients = conn_clients.clone();
        self.broadcast(clients, conn_msg.as_bytes()).await;

        let mut buf = vec![0; 1024];

        loop {
            let bytes_read = self.stream.read(&mut buf).await.map_err(|err| {
                eprintln!(
                    "ERROR: could not read message from client: {}",
                    Sesitive(err)
                )
            })?;

            let clients = conn_clients.clone();
            if bytes_read == 0 {
                let disconn_msg = format!("{} @ {}", Msg::ClientDisconnected, self.addr);
                println!("{}", style(&disconn_msg).with(Color::Red));

                self.broadcast(clients, disconn_msg.as_bytes()).await;

                // Remove this client.
                conn_clients
                    .lock()
                    .await
                    .retain(|client| client.addr != self.addr);
                break;
            } else {
                let msg = buf[..bytes_read].to_vec();
                println!("{}", Msg::Message(msg.clone()));

                self.broadcast(clients, &msg).await;
            };
        }
        Ok(())
    }
}

enum Msg {
    ClientConnected,
    ClientDisconnected,
    Message(Vec<u8>),
}

impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::ClientConnected => "Client Connected".fmt(f),
            Msg::ClientDisconnected => "Client Disonnected".fmt(f),
            Msg::Message(msg) => {
                let message = String::from_utf8(msg.clone()).unwrap();
                format!("New Message: {:?}", message).fmt(f)
            }
        }
    }
}

fn main() -> Result<()> {
    smol::block_on(async {
        let listener = TcpListener::bind(ADDRESS)
            .await
            .map_err(|err| eprintln!("ERROR: could not bind {ADDRESS}: {}", Sesitive(err)))?;
        println!("SERVER: listening at {addr}", addr = Sesitive(ADDRESS));

        let clients = Arc::new(Mutex::new(Vec::new()));

        while let Some(stream) = listener.incoming().next().await {
            match stream {
                Ok(stream) => {
                    let mut client = Client::new(stream.peer_addr().unwrap(), stream.clone());
                    clients.lock().await.push(client.clone()); // Acquire the lock

                    let clients_clone = clients.clone();
                    smol::spawn(async move { client.handle_connection(clients_clone).await })
                        .detach();
                }

                Err(err) => {
                    eprintln!("ERROR: could not accept connection: {}", Sesitive(err));
                }
            }
        }

        Ok(())
    })
}
