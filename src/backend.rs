use std::{
    fmt::Display,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    result,
    sync::{Arc, Mutex},
    thread,
};

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

fn handle_client(mut stream: TcpStream, conn_clients: Arc<Mutex<Vec<Client>>>) -> Result<()> {
    let mut buf = vec![0; 1024];

    loop {
        let bytes_read = stream.read(&mut buf).map_err(|err| {
            eprintln!(
                "ERROR: could not read message from client: {}",
                Sesitive(err)
            )
        })?;

        if bytes_read == 0 {
            println!("{}", Msg::ClientDisconnected);
            break;
        } else {
            let msg = buf[..bytes_read].to_vec();
            println!("{}", Msg::Message(msg));

            // Broadcast to all the fellow clients.
            let mut clients = conn_clients
                .lock()
                .map_err(|err| eprintln!("ERROR: could not aquire lock: {}", Sesitive(err)))?;

            for client in clients.iter_mut() {
                if stream.peer_addr().unwrap() != client.addr {
                    let _bytes_written = client.stream.write(&buf[..bytes_read]).unwrap();
                }
            }
        };
    }
    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS)
        .map_err(|err| eprintln!("ERROR: could not bind {ADDRESS}: {}", Sesitive(err)))?;
    println!("SERVER: listening at {addr}", addr = Sesitive(ADDRESS));

    let clients = Arc::new(Mutex::new(Vec::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let addr = stream.peer_addr().unwrap();

                let mut cls = clients
                    .lock()
                    .map_err(|err| eprintln!("ERROR: could not aquire lock: {}", Sesitive(err)))?;
                cls.push(Client::new(addr, stream.try_clone().unwrap()));

                println!("{} @ {}", Msg::ClientConnected, addr);
                let conn_clients = Arc::clone(&clients);
                thread::spawn(|| handle_client(stream, conn_clients));
            }

            Err(err) => {
                eprintln!("ERROR: could not accept connedction: {}", Sesitive(err));
            }
        }
    }

    Ok(())
}
