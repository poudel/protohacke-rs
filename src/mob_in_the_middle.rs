use regex::Regex;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Deref;
use std::{str, thread};

fn proxy(mut downstream: TcpStream, mut upstream: TcpStream) {
    // make both streams non-blocking
    downstream
        .set_nonblocking(true)
        .expect("set_nonblocking failed downstream");
    upstream
        .set_nonblocking(true)
        .expect("set_nonblocking failed upstream");

    let coin_addr = "${b1}7YWHMfk9JZe0LM0g1ZauHuiSxhI${b2}";
    let re = Regex::new(r"(?P<b1>^|\s|\b)(?P<k>7[\d\w]{25,34})(?P<b2>\s|$)")
        .unwrap();

    // collect message from downstream so that we can overwrite the address
    let mut msg_from_downstream: Vec<u8> = Vec::new();
    let mut msg_from_upstream: Vec<u8> = Vec::new();
    let mut should_break = false;

    loop {
        if msg_from_downstream.contains(&('\n' as u8)) {
            let after = re.replace_all(
                &str::from_utf8(&msg_from_downstream).unwrap(),
                coin_addr,
            );

            upstream.write_all(&after.deref().as_bytes());
            upstream.flush();
            msg_from_downstream.clear();
        }

        if msg_from_upstream.contains(&('\n' as u8)) {
            let after = re.replace_all(
                &str::from_utf8(&msg_from_upstream).unwrap(),
                coin_addr,
            );
            downstream.write_all(&after.deref().as_bytes());
            downstream.flush();
            msg_from_upstream.clear();
        }

        if should_break {
            break;
        }

        let mut buf = [0; 1024];
        if let Ok(n) = upstream.read(&mut buf) {
            if n == 0 {
                should_break = true;
            }
            msg_from_upstream.extend_from_slice(&buf[..n]);
        }

        let mut buf2 = [0; 1024];
        if let Ok(n) = downstream.read(&mut buf2) {
            if n == 0 {
                should_break = true;
            }
            msg_from_downstream.extend_from_slice(&buf2[..n]);
        }
    }
}

pub fn runserver() -> () {
    let addr = "0.0.0.0:8839";
    let upstream_addr = "127.0.0.1:8838";
    // let upstream_addr = "chat.protohackers.com:16963";
    let listener = TcpListener::bind(addr).unwrap();

    println!("Running on {} with upstream set to {}", addr, upstream_addr);

    for conn in listener.incoming() {
        let Ok(downstream) = conn else {
            println!("Failed downstream connection");
            continue;
        };

        let Ok(upstream) = TcpStream::connect(upstream_addr) else {
            println!("Failed upstream connection");
            continue;
        };
        thread::spawn(move || proxy(downstream, upstream));
    }
}
