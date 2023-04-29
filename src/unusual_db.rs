use std::collections::HashMap;
use std::net::UdpSocket;
use std::str::from_utf8;

fn process_message(db: &mut HashMap<String, String>, msg: &str) -> Option<String> {
    println!("db: {:?}", db);
    if let Some((first, second)) = msg.split_once('=') {
        if first != "version" {
            db.insert(first.to_string(), second.to_string());
        }
    } else {
        return Some(format!("{}={}", msg, db.get(msg).cloned()?));
    }
    None
}

pub fn runserver() -> () {
    let mut db: HashMap<String, String> = HashMap::new();
    db.insert("version".to_string(), "KP's KV 0.1".to_string());

    let addr = "0.0.0.0:8838";
    let socket = UdpSocket::bind(addr).unwrap();
    println!("'Listening' to {}", addr);

    loop {
        let mut buf = [0; 1000];

        match socket.recv_from(&mut buf) {
            Ok((num_recvd, src_addr)) => {
                let msg = from_utf8(&buf[..num_recvd]).unwrap();
                if let Some(resp) = process_message(&mut db, &msg) {
                    println!("Found: {}", resp);
                    socket.send_to(&resp.as_bytes(), src_addr);
                }
            }
            Err(e) => {
                eprintln!("Err reading: {}", e);
            }
        };
    }
}
