use crossterm::style::{style, Color, Stylize};
use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    lock::Mutex,
    net::{SocketAddr, TcpListener, TcpStream},
    stream::StreamExt,
};
use std::{error::Error, fmt::Display, sync::Arc};

const ADDRESS: &str = "127.0.0.1:3000";
const SAFE_MODE: bool = false;

struct Sensitive<T>(T);

impl<T> Display for Sensitive<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Sensitive(data) = self;
        if SAFE_MODE {
            "[REDACTED]".fmt(f)
        } else {
            data.fmt(f)
        }
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
                let message = String::from_utf8_lossy(msg);
                format!("New Message: {:?}", message).fmt(f)
            }
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
        let mut clients = conn_clients.lock().await;

        for client in clients.iter_mut().filter(|client| client.addr != self.addr) {
            if let Err(err) = client.stream.write(msg).await {
                eprintln!(
                    "ERROR: could not write to client {}: {}",
                    client.addr,
                    Sensitive(err)
                );
            }
        }
    }

    async fn handle_connection(
        &mut self,
        conn_clients: Arc<Mutex<Vec<Client>>>,
    ) -> Result<(), Box<dyn Error>> {
        let conn_msg = format!("{} @ {}", Msg::ClientConnected, self.addr);
        println!("{}", style(&conn_msg).with(Color::Green));

        self.broadcast(conn_clients.clone(), conn_msg.as_bytes())
            .await;

        let mut buf = vec![0; 1024];

        loop {
            let bytes_read = self.stream.read(&mut buf).await.inspect_err(|err| {
                eprintln!(
                    "ERROR: could not read message from client: {}",
                    Sensitive(err)
                )
            })?;

            match bytes_read {
                0 => {
                    let disconn_msg = format!("{} @ {}", Msg::ClientDisconnected, self.addr);
                    println!("{}", style(&disconn_msg).with(Color::Red));

                    self.broadcast(conn_clients.clone(), disconn_msg.as_bytes())
                        .await;

                    // Remove this client.
                    conn_clients
                        .lock()
                        .await
                        .retain(|client| client.addr != self.addr);
                    break;
                }
                _ => {
                    let msg = buf[..bytes_read].to_vec();
                    println!("{}", Msg::Message(msg.clone()));

                    self.broadcast(conn_clients.clone(), &msg).await;
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    smol::block_on(async {
        let listener = TcpListener::bind(ADDRESS)
            .await
            .inspect_err(|err| eprintln!("ERROR: could not bind {ADDRESS}: {}", Sensitive(err)))?;
        println!("SERVER: listening at {addr}", addr = Sensitive(ADDRESS));

        let clients = Arc::new(Mutex::new(Vec::new()));

        while let Some(stream) = listener.incoming().next().await {
            match stream {
                Ok(stream) => match stream.peer_addr() {
                    Ok(addr) => {
                        let mut client = Client::new(addr, stream);
                        clients.lock().await.push(client.clone());

                        let conn_clients = clients.clone();
                        smol::spawn(async move {
                            if let Err(err) = client.handle_connection(conn_clients).await {
                                eprintln!(
                                    "ERROR: client {} disconnected with an error: {}",
                                    client.addr,
                                    Sensitive(err)
                                );
                            }
                        })
                        .detach();
                    }
                    Err(err) => {
                        eprintln!("ERROR: could not get peer address: {}", Sensitive(err));
                    }
                },

                Err(err) => {
                    eprintln!("ERROR: could not accept connection: {}", Sensitive(err));
                }
            }
        }

        Ok(())
    })
}
