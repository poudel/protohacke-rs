use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn mean(v: Vec<&i32>) -> i32 {
    if v.is_empty() {
        return 0;
    }

    let total: i64 = v.iter().map(|&num| *num as i64).sum();
    let mean = total as f64 / (v.len() as f64);

    return mean.round() as i32;
}

fn handle_client(mut stream: TcpStream) {
    let mut prices: HashMap<i32, i32> = HashMap::new();

    println!("Reading....");

    loop {
        let mut chunk = [0; 9];
        stream.read_exact(&mut chunk).unwrap();

        let first: i32 = i32::from_be_bytes(chunk[1..5].try_into().unwrap());
        let second: i32 = i32::from_be_bytes(chunk[5..9].try_into().unwrap());

        match chunk[0] {
            73 => {
                // Insert

                if prices.contains_key(&first) {
                    println!("Duplicate key, breaking!");
                    break;
                } else {
                    prices.insert(first, second);
                    println!("Inserted, {:?}, {:?}", first, second);
                }
            }

            81 => {
                // Query

                if first > second {
                    stream.write(&i32::to_be_bytes(0));
                    stream.flush();
                    println!("Mintime > Maxtime, {:?}, {:?}", first, second);
                    continue;
                }

                let mut sumprice: Vec<&i32> = Vec::new();
                for (key, value) in prices.iter() {
                    if (&first <= key) && (&second >= key) {
                        sumprice.push(value);
                    }
                }

                let mean: i32 = mean(sumprice);
                stream.write(&i32::to_be_bytes(mean));
                stream.flush();
                println!("Returned mean: {:?}", mean);
            }

            _ => {
                println!("Invalid message: {:?}", chunk);
                break;
            }
        }
    }
    println!("Stop!");
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
                    handle_client(stream);
                });
            }

            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mean() {
        assert_eq!(mean(vec![&1, &2, &3, &4]), 3);
    }
}
