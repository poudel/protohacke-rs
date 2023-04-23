use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut buf = Vec::new();

    loop {
        match stream.read_to_end(&mut buf) {
            Ok(_) => {
                if let Err(e) = stream.write_all(&buf) {
                    eprintln!("Error writing to socket: {:?}", e);
                    return;
                }
            }

            Err(e) => {
                eprintln!("Error reading from socket: {:?}", e);
                return;
            }
        }
    }
}

pub fn runserver() -> () {
    let addr = "0.0.0.0:8838";

    // unwrap() will crash if bind returns an Error
    // but this is tolerable
    let listener = TcpListener::bind(addr).unwrap();

    println!("Running on {}", addr);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream);
                });
            }

            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }
    }
}
