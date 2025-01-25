#![allow(unused_imports)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                let mut buffer = [0; 512];
                match _stream.read(&mut buffer) {
                    Ok(n) => {
                        println!("{}", String::from_utf8_lossy(&buffer[..n]));
                        _stream.write_all(b"+PONG\r\n").unwrap();
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                };
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
