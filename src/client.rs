use chrono::Local;
use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyModifiers},
    terminal::{self, ClearType},
    QueueableCommand,
};
use smol::{
    channel::Sender,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use std::{
    error::Error,
    io::{stdout, Stdout, Write},
};

const ADDRESS: &str = "127.0.0.1:3000";

struct Message {
    time: String,
    content: String,
}

impl Message {
    fn new(content: String) -> Self {
        Message {
            time: Local::now().format("%H:%M").to_string(),
            content,
        }
    }
}

enum AppEvent {
    Crossterm(Event),
    Network(String),
    NetworkError(String),
    InputError(String),
}

async fn stream_handler(mut stream: TcpStream, sender: Sender<AppEvent>) {
    let mut buf = vec![0; 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    let _ = sender
                        .send(AppEvent::NetworkError(
                            "Connection closed by server".to_string(),
                        ))
                        .await;
                    break;
                }
                let msg = String::from_utf8_lossy(&buf[..bytes_read]).to_string();
                let _ = sender.send(AppEvent::Network(msg)).await;
            }
            Err(e) => {
                let _ = sender
                    .send(AppEvent::NetworkError(format!("Read error: {}", e)))
                    .await;
                break;
            }
        }
    }
}

async fn input_handler(sender: Sender<AppEvent>) {
    loop {
        match smol::unblock(crossterm::event::read).await {
            Ok(event) => {
                let _ = sender.send(AppEvent::Crossterm(event)).await;
            }
            Err(e) => {
                let _ = sender
                    .send(AppEvent::InputError(format!("Input error: {}", e)))
                    .await;
                break;
            }
        }
    }
}

fn render(
    stdout: &mut Stdout,
    (width, height): (u16, u16),
    (messages, prompt): (&[Message], &str),
) -> Result<(), Box<dyn Error>> {
    stdout.queue(terminal::Clear(ClearType::All))?;

    let skip_count = if messages.len() <= (height - 2) as usize {
        0
    } else {
        messages.len() - ((height - 2) as usize)
    };

    for (row, message) in messages.iter().skip(skip_count).enumerate() {
        let msg = format!("[{}] {}", message.time, message.content);
        let max_len = msg.len().min(width as usize);
        stdout
            .queue(cursor::MoveTo(0, row as u16))?
            .write_all(msg[..max_len].as_bytes())?;
    }

    stdout
        .queue(cursor::MoveTo(0, height - 2))?
        .write_all("─".repeat(width as usize).as_bytes())?;

    stdout
        .queue(cursor::MoveTo(0, height - 1))?
        .write_all(">>".as_bytes())?;

    let min_len = if prompt.len() <= width as usize {
        0
    } else {
        prompt.len() - width as usize + 1
    };
    stdout
        .queue(cursor::MoveTo(3, height - 1))?
        .write_all(prompt[min_len..].as_bytes())?;

    stdout.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    smol::block_on(async {
        let mut stream = TcpStream::connect(ADDRESS).await.inspect_err(|err| {
            eprintln!("ERROR: could not bind {ADDRESS}: {}", err);
            eprintln!("WARN: ensure that the server is running.");
        })?;
        let (sender, receiver) = smol::channel::unbounded::<AppEvent>();

        let read_stream = stream.clone();
        let io_sender = sender.clone();
        smol::spawn(async move {
            stream_handler(read_stream, io_sender).await;
        })
        .detach();

        smol::spawn(async move {
            let _ = input_handler(sender).await;
        })
        .detach();

        // Terminal UI Startup
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.queue(terminal::Clear(ClearType::All))?.flush()?;

        let (mut width, mut height) = terminal::size()?;
        let (mut messages, mut prompt) = (Vec::new(), String::new());
        messages.push(Message::new("Connected to server".to_string()));
        render(&mut stdout, (width, height), (&messages, &prompt))?;

        loop {
            if let Ok(event) = receiver.recv().await {
                match event {
                    AppEvent::Network(msg) => {
                        messages.push(Message::new(msg));
                    }
                    AppEvent::NetworkError(err) => {
                        messages.push(Message::new(format!("Network error: {}", err)));
                        break;
                    }
                    AppEvent::InputError(err) => {
                        messages.push(Message::new(format!("Input error: {}", err)));
                        break;
                    }
                    AppEvent::Crossterm(event) => match event {
                        Event::Resize(new_width, new_height) => {
                            width = new_width;
                            height = new_height;
                        }
                        Event::Key(key_event) => match key_event.code {
                            KeyCode::Enter => {
                                if !prompt.is_empty() {
                                    stream.write_all(prompt.as_bytes()).await?;
                                    messages.push(Message::new(prompt.clone()));
                                    prompt.clear();
                                }
                            }
                            KeyCode::Backspace => {
                                prompt.pop();
                            }
                            KeyCode::Char(ch) => {
                                if ch == 'c' && key_event.modifiers.contains(KeyModifiers::CONTROL)
                                {
                                    break;
                                }
                                prompt.push(ch);
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                render(&mut stdout, (width, height), (&messages, &prompt))?;
            }
        }

        // Terminal UI Cleanup.
        stdout
            .queue(terminal::Clear(ClearType::All))?
            .queue(cursor::MoveTo(0, 0))?
            .flush()?;
        terminal::disable_raw_mode()?;
        Ok(())
    })
}
