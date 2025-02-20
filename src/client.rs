use chrono::Local;
use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode, KeyModifiers},
    terminal::{self, ClearType},
    QueueableCommand,
};
use std::{
    error::Error,
    io::{stdout, Read, Write},
    net::TcpStream,
    thread,
    time::Duration,
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

fn main() -> Result<(), Box<dyn Error>> {
    // Enable raw mode for better control over the terminal
    terminal::enable_raw_mode()?;
    let mut stream = TcpStream::connect(ADDRESS)?;
    stream.set_nonblocking(true)?;

    let mut stdout = stdout();
    let (mut width, mut height) = terminal::size().unwrap();

    let mut messages = Vec::new();
    messages.push(Message::new("Connected to server".to_string()));
    let mut prompt = String::new();

    let mut quit = false;
    while !quit {
        while poll(Duration::ZERO)? {
            match read()? {
                Event::Resize(new_width, new_height) => {
                    width = new_width;
                    height = new_height;
                }

                Event::Key(event) => match event.code {
                    KeyCode::Enter => {
                        if !prompt.is_empty() {
                            let msg = Message {
                                time: Local::now().format("%H:%M").to_string(),
                                content: prompt.clone(),
                            };
                            messages.push(msg);
                            let bytes_written = stream.write(prompt.as_bytes())?;
                            println!("Bytes written: {}", bytes_written);
                            prompt.clear();
                        }
                    }

                    KeyCode::Backspace => {
                        prompt.pop();
                    }

                    KeyCode::Left => {
                        cursor::MoveLeft(1);
                    }

                    KeyCode::Char(ch) => {
                        if ch == 'c' && event.modifiers.contains(KeyModifiers::CONTROL) {
                            quit = true;
                        } else {
                            prompt.push(ch);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        //Listening for peer messages.
        let mut buf = vec![0; 1024];

        if let Ok(bytes_read) = stream.read(&mut buf) {
            if bytes_read != 0 {
                let peer_msg = String::from_utf8(buf[..bytes_read].to_vec()).unwrap();
                messages.push(Message::new(peer_msg));
            }
        }

        // Clear the terminal
        stdout.queue(terminal::Clear(ClearType::All))?;

        let skip_count = if messages.len() <= (height - 2) as usize {
            0
        } else {
            messages.len() - ((height - 2) as usize)
        };
        // Render the chat
        for (row, message) in messages.iter().skip(skip_count).enumerate() {
            let msg = format!("[{}] {}", message.time, message.content);
            let max_len = if msg.len() <= width as usize {
                msg.len()
            } else {
                width as usize
            };

            stdout
                .queue(cursor::MoveTo(0, row as u16))?
                .write(msg[..max_len].as_bytes())?;
        }

        stdout
            .queue(cursor::MoveTo(0, height - 2))?
            .write(get_border(width).as_bytes())?;

        let min_len = if prompt.len() <= width as usize {
            0
        } else {
            prompt.len() - width as usize + 1
        };
        stdout
            .queue(cursor::MoveTo(0, height - 1))?
            .write(prompt[min_len..].as_bytes())?;

        stdout.flush()?;

        // Ensures 30FPS
        thread::sleep(Duration::from_millis(33));
    }

    // Disable raw mode before exiting
    terminal::disable_raw_mode()?;
    Ok(())
}

fn get_border(width: u16) -> String {
    "─".repeat(width as usize)
}
