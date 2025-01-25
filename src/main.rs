#![allow(unused_imports)]

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                handle_client(&mut _stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(stream: &mut TcpStream) {
    let mut buffer = [0; 512];

    match stream.read(&mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => {
            let input = String::from_utf8_lossy(&buffer[..bytes_read]);

            if input.trim().contains("PING") {
                stream.write_all(b"+PONG\r\n").unwrap();
            }
        }
        Ok(_) => {
            println!("Client disconnected.");
        }
        Err(e) => {
            println!("error: {}", e);
        }
    };
}
