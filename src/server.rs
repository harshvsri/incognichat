use smol::{
    channel::{Receiver, Sender},
    io::{AsyncReadExt, AsyncWriteExt},
    net::{SocketAddr, TcpListener, TcpStream},
    stream::StreamExt,
};
use std::{error::Error, fmt::Display};

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

#[derive(Debug, Clone)]
struct Client {
    addr: SocketAddr,
    stream: TcpStream,
}

impl Client {
    fn new(addr: SocketAddr, stream: TcpStream) -> Self {
        Client { addr, stream }
    }

    async fn handler(&mut self, sender: Sender<ServerAction>) -> Result<(), Box<dyn Error>> {
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
                    sender
                        .send(ServerAction::ClientDisconnected { addr: self.addr })
                        .await?;
                    break;
                }
                _ => {
                    let msg = buf[..bytes_read].to_vec();
                    sender
                        .send(ServerAction::Message {
                            addr: self.addr,
                            msg,
                        })
                        .await?;
                }
            }
        }
        Ok(())
    }
}

enum ServerAction {
    ClientConnected { client: Client },
    ClientDisconnected { addr: SocketAddr },
    Message { addr: SocketAddr, msg: Vec<u8> },
}

async fn broadcast_handler(reciever: Receiver<ServerAction>) {
    let mut clients: Vec<Client> = Vec::new();

    loop {
        if let Ok(action) = reciever.recv().await {
            match action {
                ServerAction::ClientConnected { client } => {
                    let connected_msg = format!("{} has joined the chat", client.addr);
                    println!("{connected_msg}");

                    for client in clients.iter_mut() {
                        if let Err(e) = client.stream.write_all(connected_msg.as_bytes()).await {
                            eprintln!("Failed to write to {}: {}", client.addr, e);
                        }
                    }

                    clients.push(client);
                }
                ServerAction::ClientDisconnected { addr } => {
                    clients.retain(|client| client.addr != addr);

                    let disconnected_msg = format!("{} has left the chat", addr);
                    println!("{disconnected_msg}");

                    for client in clients.iter_mut() {
                        if let Err(e) = client.stream.write_all(disconnected_msg.as_bytes()).await {
                            eprintln!("Failed to write to {}: {}", client.addr, e);
                        }
                    }
                }
                ServerAction::Message { addr, msg } => {
                    let full_msg = format!("{}: {}", addr, String::from_utf8_lossy(&msg));
                    println!("{full_msg}");

                    for client in clients.iter_mut().filter(|client| client.addr != addr) {
                        if let Err(e) = client.stream.write_all(full_msg.as_bytes()).await {
                            eprintln!("Failed to write to {}: {}", client.addr, e);
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    smol::block_on(async {
        let listener = TcpListener::bind(ADDRESS)
            .await
            .inspect_err(|err| eprintln!("ERROR: could not bind {ADDRESS}: {}", Sensitive(err)))?;
        println!("SERVER: listening at {addr}", addr = Sensitive(ADDRESS));

        let (sender, reciever) = smol::channel::unbounded::<ServerAction>();

        smol::spawn(async move { broadcast_handler(reciever).await }).detach();

        while let Some(stream) = listener.incoming().next().await {
            match stream {
                Ok(stream) => match stream.peer_addr() {
                    Ok(addr) => {
                        let mut client = Client::new(addr, stream);

                        sender
                            .send(ServerAction::ClientConnected {
                                client: client.clone(),
                            })
                            .await?;

                        let sender = sender.clone();
                        smol::spawn(async move {
                            if let Err(err) = client.handler(sender).await {
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
