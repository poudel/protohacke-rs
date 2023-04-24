use serde::{Deserialize, Serialize};
use serde_json::{json, Number};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn num_is_prime(num: i64) -> bool {
    if num == 2 {
        return true;
    }

    if (num <= 1) || (num % 2 == 0) {
        return false;
    }

    let limit = (num as f64).sqrt().ceil() as i64;
    for i in 2..=limit {
        if num % i == 0 {
            return false;
        }
    }
    return true;
}

#[derive(Serialize, Deserialize)]
struct ProtoRequest {
    method: String,
    number: Number,
}

enum ProtoResult {
    IsPrime,
    IsNotPrime,
    Malformed,
}

fn do_math_and_stuff(data: &String) -> ProtoResult {
    match serde_json::from_str::<ProtoRequest>(&data) {
        Ok(v) => {
            if v.method != String::from("isPrime") {
                return ProtoResult::Malformed;
            }

            if !v.number.is_i64() {
                return ProtoResult::IsNotPrime;
            }

            let number: i64 = v.number.as_i64().unwrap();

            if num_is_prime(number) {
                return ProtoResult::IsPrime;
            }

            return ProtoResult::IsNotPrime;
        }
        Err(_) => ProtoResult::Malformed,
    }
}

fn handle_prime_client(stream: TcpStream) {
    let reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    for row in reader.lines() {
        match row {
            Ok(data) => match do_math_and_stuff(&data) {
                ProtoResult::IsPrime => {
                    let output = json!({
                        "method": "isPrime",
                        "prime": true
                    });
                    writer.write(&output.to_string().as_bytes()).unwrap();
                    writer.write(b"\n").unwrap();
                    writer.flush().unwrap();
                }
                ProtoResult::IsNotPrime => {
                    let output = json!({
                        "method": "isPrime",
                        "prime": false
                    });
                    writer.write(&output.to_string().as_bytes()).unwrap();
                    writer.write(b"\n").unwrap();
                    writer.flush().unwrap();
                }
                ProtoResult::Malformed => {
                    writer.write(b"malformed\n").unwrap();
                    break;
                }
            },
            Err(e) => {
                eprintln!("Failed to read from socket: {:?}", e);
                writer.write(b"Noooo...\n").unwrap();
                break;
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
        println!("Connected!");
        match conn {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_prime_client(stream);
                });
            }

            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }
    }
}
