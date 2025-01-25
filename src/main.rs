#![allow(unused_imports)]

use std::str;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return, // Connection closed
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if let Ok(input) = str::from_utf8(&buf[..n]) {
                    let input = input.trim();

                    match parse_command(input) {
                        Some(response) => {
                            if let Err(e) = socket.write_all(response.as_bytes()).await {
                                eprintln!("failed to write to socket; err = {:?}", e);
                                return;
                            }
                        }
                        None => {
                            let error_msg = "-ERR Ungültiges Kommando\r\n";
                            socket.write_all(error_msg.as_bytes()).await.unwrap();
                            return;
                        }
                    }
                };

                if let Err(e) = socket.write_all(b"+PONG\r\n").await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}

fn parse_command(input: &str) -> Option<String> {
    let mut parts = input.split_whitespace();
    let command = parts.next()?;
    let args = parts.next()?;

    match command {
        "ECHO" => {
            Some(format!("$3\r\n{}\r\n", args))
        }

        _ => None,
    }
}