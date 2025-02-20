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

struct Client {
    addr: SocketAddr,
    stream: TcpStream,
}

impl Client {
    fn new(addr: SocketAddr, stream: TcpStream) -> Self {
        Client { addr, stream }
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

async fn handle_client(mut stream: TcpStream, conn_clients: Arc<Mutex<Vec<Client>>>) -> Result<()> {
    let mut buf = vec![0; 1024];

    loop {
        let bytes_read = stream.read(&mut buf).await.map_err(|err| {
            eprintln!(
                "ERROR: could not read message from client: {}",
                Sesitive(err)
            )
        })?;

        if bytes_read == 0 {
            println!(
                "{}",
                style(format!("{}", Msg::ClientDisconnected)).with(Color::Red)
            );

            // Remove this client.
            let addr = stream.peer_addr().unwrap();
            conn_clients
                .lock()
                .await
                .retain(|client| client.addr != addr);
            break;
        } else {
            let msg = buf[..bytes_read].to_vec();
            println!("{}", Msg::Message(msg.clone()));

            // Broadcast to all the fellow clients.
            let mut clients = conn_clients.lock().await;

            for client in clients.iter_mut() {
                if stream.peer_addr().unwrap() != client.addr {
                    let _bytes_written = client.stream.write(&msg).await.unwrap_or_default();
                }
            }
        };
    }
    Ok(())
}

fn main() -> Result<()> {
    smol::block_on(async {
        let listener = TcpListener::bind(ADDRESS)
            .await
            .map_err(|err| eprintln!("ERROR: could not bind {ADDRESS}: {}", Sesitive(err)))?;
        println!("SERVER: listening at {addr}", addr = Sesitive(ADDRESS));

        let clients = Arc::new(Mutex::new(Vec::new()));
        //let exe = Executor::new();
        while let Some(stream) = listener.incoming().next().await {
            match stream {
                Ok(stream) => {
                    let addr = stream.peer_addr().unwrap();
                    {
                        let mut clients = clients.lock().await; // Acquire the lock
                        clients.push(Client::new(addr, stream.clone())); // Update shared state
                    }

                    println!(
                        "{}",
                        style(format!("{} @ {}", Msg::ClientConnected, addr)).with(Color::Green)
                    );

                    let clients_clone = clients.clone();
                    smol::spawn(handle_client(stream, clients_clone)).detach();
                }

                Err(err) => {
                    eprintln!("ERROR: could not accept connection: {}", Sesitive(err));
                }
            }
        }

        Ok(())
    })
}
