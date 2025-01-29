use chrono::Local;
use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode, KeyModifiers},
    terminal::{self, ClearType},
    QueueableCommand,
};
use std::{
    io::{self, stdout, Write},
    thread,
    time::Duration,
};

struct Message {
    time: String,
    content: String,
}

fn main() -> io::Result<()> {
    // Enable raw mode for better control over the terminal
    terminal::enable_raw_mode()?;

    let mut stdout = stdout();
    let (mut width, mut height) = terminal::size().unwrap();
    let mut messages = Vec::new();
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

        thread::sleep(Duration::from_millis(33));
    }

    // Disable raw mode before exiting
    terminal::disable_raw_mode()?;
    Ok(())
}

fn get_border(width: u16) -> String {
    "─".repeat(width as usize)
}
