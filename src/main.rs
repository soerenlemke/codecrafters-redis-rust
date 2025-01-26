#![allow(unused_imports)]

mod resp_parser;

use crate::resp_parser::value::{parse_message, Value};
use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
                    let response = parse_and_respond(input);

                    if let Err(e) = socket.write_all(response.as_bytes()).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return;
                    }
                }
            }
        });
    }
}

fn parse_and_respond(input: &str) -> String {
    match parse_message(input) {
        Ok((_, value)) => match value {
            Value::Array(elements) => {
                if elements.is_empty() {
                    return "-ERR leeres Kommando\r\n".to_string();
                }

                let cmd = &elements[0];
                match cmd {
                    Value::BulkString(cmd_str) | Value::SimpleString(cmd_str) => {
                        let cmd_upper = cmd_str.to_ascii_uppercase();

                        match cmd_upper.as_str() {
                            "PING" => "+PONG\r\n".to_string(),
                            "ECHO" => {
                                if elements.len() < 2 {
                                    return "-ERR ECHO braucht ein Argument\r\n".to_string();
                                }
                                match &elements[1] {
                                    Value::BulkString(arg) => {
                                        format!("${}\r\n{}\r\n", arg.len(), arg)
                                    }
                                    _ => "-ERR ECHO erwartet eine BulkString-Argument\r\n"
                                        .to_string(),
                                }
                            }
                            other => {
                                // Unbekanntes Kommando
                                format!("-ERR Unbekanntes Kommando: {}\r\n", other)
                            }
                        }
                    }
                    _ => "-ERR Erstes Array-Element muss der Befehl sein\r\n".to_string(),
                }
            }

            Value::SimpleString(s) => {
                format!("+Hallo, du hast '{}' geschickt\r\n", s)
            }
            Value::BulkString(s) => {
                format!("+BulkString: {}\r\n", s)
            }
            Value::Integer(i) => {
                format!("+Integer: {}\r\n", i)
            }
            _ => "-ERR Nicht unterstütztes Format\r\n".to_string(),
        },
        Err(e) => {
            eprintln!("Parsing error: {:?}", e);
            "-ERR Ungültiges RESP\r\n".to_string()
        }
    }
}
