use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

enum ChatMsg {
    Joined(TcpStream, String),
    Left(String),
    Message(String, String),
}

fn handle_client(stream: TcpStream, send: mpsc::Sender<ChatMsg>) {
    let reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    let mut username: String = String::from("<anon>");

    // ask for the username first
    writer.write(&String::from("Welcome to budgetchat! What shall I call you?\n").as_bytes());
    writer.flush();

    for row in reader.lines() {
        match row {
            Ok(msg) => {
                if username != "<anon>" {
                    send.send(ChatMsg::Message(username.clone(), msg));
                    continue;
                }

                // set the username if not done yet but validate it first
                if msg.len() == 0 || !msg.chars().all(char::is_alphanumeric) {
                    writer.write(format!("Invalid username, {:?}\n", msg).as_bytes());
                    writer.flush();
                    break;
                }

                username = msg;
                send.send(ChatMsg::Joined(
                    stream.try_clone().unwrap(),
                    username.clone(),
                ));
                continue;
            }
            Err(e) => {
                eprintln!("Error reading! {}", e);
            }
        }
    }

    if username != "<anon>" {
        send.send(ChatMsg::Left(username));
    }

    println!("Stopped!");
}

fn chatmaster(recv: mpsc::Receiver<ChatMsg>) {
    let mut registry: HashMap<String, TcpStream> = HashMap::new();

    for chat_msg in recv.iter() {
        match chat_msg {
            ChatMsg::Joined(mut stream, username) => {
                let usernames = registry.keys().fold(String::new(), |x, y| {
                    if x.is_empty() {
                        y.to_string()
                    } else {
                        format!("{}, {}", x, y)
                    }
                });
                stream.write(format!("* The room contains: {}\n", usernames).as_bytes());

                registry.insert(username.clone(), stream);

                let outgoing = format!("* {} has joined the room\n", username);
                println!("{}", outgoing);
                for (u, mut s) in &registry {
                    if u == &username {
                        continue;
                    }
                    s.write(&outgoing.as_bytes());
                    s.flush();
                }
            }

            ChatMsg::Left(username) => {
                if !registry.contains_key(&username) {
                    continue;
                }

                registry.remove(&username);

                for (u, mut s) in &registry {
                    let outgoing = format!("* {} has left the room\n", username);

                    println!("{}", outgoing);
                    s.write(outgoing.as_bytes());
                    s.flush();
                }
            }
            ChatMsg::Message(username, msg) => {
                for (u, mut s) in &registry {
                    if u == &username {
                        continue;
                    }

                    s.write(format!("[{}] {}\n", username, msg).as_bytes());
                    s.flush();
                }
            }
        }
    }
}

pub fn runserver() -> () {
    let addr = "0.0.0.0:8838";
    let listener = TcpListener::bind(addr).unwrap();

    println!("Running on {}", addr);

    let (send, recv) = mpsc::channel::<ChatMsg>();

    // spawn a separate receiver thread aka chatmaster
    thread::spawn(move || chatmaster(recv));

    for conn in listener.incoming() {
        println!("Connected!");

        match conn {
            Ok(stream) => {
                let cloned_send = send.clone();
                thread::spawn(move || handle_client(stream, cloned_send));
            }

            Err(e) => {
                eprintln!("Connection failed: {:?}", e);
            }
        }
    }
}
