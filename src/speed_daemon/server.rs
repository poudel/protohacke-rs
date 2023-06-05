use std::collections::HashMap;
use std::slice::Chunks;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{
    AsyncReadExt, AsyncWriteExt, BufReader, BufStream, BufWriter, ReadHalf,
    WriteHalf,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{mpsc, Mutex};

use anyhow::{bail, Result};
use tokio::time;

use itertools::Itertools;

use crate::speed_daemon::message::*;


pub struct Client {
    idx: usize,
    reader: XReader,
    sender: XSender,
    tm_sender: RSender,
    has_heartbeat: bool,
}


impl Client {
    pub fn log(&self, s: String) {
        println!("{0:?} {s}", self.idx);
    }

    pub fn new(idx: usize, stream: TcpStream, tm_sender: RSender) -> Self {
        let (sender, reader) = Client::split_stream(idx.clone(), stream);

        Self {
            idx: idx,
            reader: reader,
            sender: sender,
            tm_sender: tm_sender,
            has_heartbeat: false,
        }
    }

    /// Split the stream and also start a mpsc channel receiver
    fn split_stream(idx: usize, mut stream: TcpStream) -> (XSender, XReader) {
        let (read_half, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);
        // let mut writer = BufWriter::new(write_half);

        let (sender, mut receiver): (XSender, XReceiver) = channel(100);

        // Start a new task that listens to mpsc messages
        // and writes them to the tcp stream.
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Message::Encoded(t, msg_to_send) => {
                        writer.write_all(&msg_to_send).await;
                        writer.flush().await;
                        // println!("[{idx}] {t} Sent: {msg_to_send:?}");
                    }
                    Message::Terminate => {
                        writer.shutdown();
                        receiver.close();
                        break;
                    }
                }
            }
        });

        (sender, reader)
    }

    async fn become_dispatcher(&mut self) -> Result<()> {
        let numroads = self.reader.read_u8().await?;
        let mut roads = Vec::with_capacity(numroads.into());
        for _ in 0..numroads {
            roads.push(self.reader.read_u16().await?);
        }

        self.log(format!("Dispatcher: {roads:?}"));

        self.tm_sender
            .send(Registration::NewDispatcher(roads, self.sender.clone()))
            .await?;

        loop {
            let first_byte = self.reader.read_u8().await?;
            match first_byte {
                constants::WANTHEARTBEAT => {
                    self.beat_the_heart().await?;
                }
                _ => bail!("Dispatcher -> {first_byte}"),
            }
        }

        Ok(())
    }

    async fn read_camera(&mut self) -> Result<Camera> {
        Ok(Camera {
            road: self.reader.read_u16().await?,
            mile: self.reader.read_u16().await?,
            limit: self.reader.read_u16().await?,
        })
    }

    async fn read_plate(&mut self) -> Result<String> {
        let plate_length: u8 = self.reader.read_u8().await?;
        let mut plate = Vec::with_capacity(plate_length.into());
        for _ in 0..plate_length {
            plate.push(self.reader.read_u8().await?);
        }
        Ok(String::from_utf8_lossy(&plate).to_string())
    }


    async fn become_camera(&mut self) -> Result<()> {
        let cam = self.read_camera().await?;
        self.log(format!("Camera: {cam:?}"));

        loop {
            let first_byte = self.reader.read_u8().await?;

            match first_byte {
                constants::WANTHEARTBEAT => {
                    self.beat_the_heart().await?;
                }
                constants::PLATE => {
                    let plate = self.read_plate().await?;
                    let timestamp = self.reader.read_u32().await?;
                    let ts = Timestamp::new(&cam, timestamp);
                    self.tm_sender
                        .send(Registration::NewTimestamp(plate, ts))
                        .await?;
                }
                _ => bail!("Camera -> {first_byte}"),
            }
        }

        Ok(())
    }

    async fn send_error(&mut self) -> Result<()> {
        self.sender.send(encode_error()).await;
        bail!("Done!")
    }

    pub async fn handle(&mut self) -> Result<()> {
        loop {
            let res = match self.reader.read_u8().await {
                Ok(constants::CAMERA) => self.become_camera().await,
                Ok(constants::DISPATCHER) => self.become_dispatcher().await,
                Ok(constants::WANTHEARTBEAT) => self.beat_the_heart().await,
                _ => self.send_error().await,
            };

            if let Err(e) = res {
                self.log(format!("Stopping now: {e:?}"));
                self.sender.send(encode_error()).await;
                self.sender.send(Message::Terminate).await;
                break;
            }
        }
        Ok(())
    }


    async fn beat_the_heart(&mut self) -> Result<()> {
        if self.has_heartbeat {
            bail!("Duplicate heartbeat");
        } else {
            self.has_heartbeat = true;
            let duration = self.reader.read_u32().await?;

            if duration == 0 {
                return Ok(());
            }

            let sender = self.sender.clone();
            let idx = self.idx.clone();

            tokio::spawn(async move {
                let mut counter = 0;
                loop {
                    let d = duration as f64 / 10.0;
                    let xd = Duration::from_secs_f64(d);
                    time::sleep(xd).await;
                    let res = sender
                        .send(Message::Encoded("HB".into(), vec![65u8]))
                        .await;
                    counter += 1;
                    println!("[{idx}] hb {counter}, {xd:?}, {0:?}", d);
                    if let Err(_) = res {
                        break;
                    };
                }
            });
        }
        Ok(())
    }
}


#[derive(Debug)]
pub enum Registration {
    NewDispatcher(Vec<u16>, XSender),
    NewTimestamp(String, Timestamp),
}


pub type RSender = Sender<Registration>;
pub type RReceiver = Receiver<Registration>;


/// Ticketmaster is in charge of creating and dispatching the tickets.
/// Also takes care of the state
pub async fn ticketmaster(mut receiver: RReceiver) {
    let mut dispatchers: HashMap<u16, XSender> = HashMap::new();
    let mut outbox: Vec<Ticket> = Vec::new();
    let mut timestamps: HashMap<(String, u16), Vec<Timestamp>> = HashMap::new();
    let mut sent: Vec<(String, chrono::NaiveDate)> = Vec::new();

    while let Some(rmsg) = receiver.recv().await {
        match rmsg {
            Registration::NewTimestamp(plate, timestamp) => {
                let key = (plate.clone(), timestamp.road);

                if let Some(inner_vec) = timestamps.get_mut(&key) {
                    inner_vec.push(timestamp);
                    inner_vec.sort_by_key(|ts| ts.timestamp);
                    inner_vec.dedup_by_key(|ts| ts.timestamp);

                    if inner_vec.len() < 2 {
                        continue;
                    }

                    // todo: move all of this out?
                    // todo: combinations is probably not the best way to do this
                    let comb = inner_vec.iter().combinations(2);

                    for chunk in comb {
                        let [first, second] = chunk[..] else {
                            break;
                        };

                        let distance: f64 = if second.mile > first.mile {
                            second.mile - first.mile
                        } else {
                            first.mile - second.mile
                        }
                        .into();

                        let time: f64 =
                            (second.timestamp - first.timestamp).into();
                        let speed = (distance / time) * 60.0 * 60.0 * 100.0;
                        let speed = speed as u16;
                        let limit = first.limit * 100;

                        // todo: remove timestamps that are no longer necessary
                        if speed > limit {
                            // println!("Create ticket: {first:?}, {second:?}");
                            // println!("Speed: {speed:?}, Limit: {0:?}", limit);

                            let mut ticket = Ticket::new(
                                plate.clone(),
                                first.clone(),
                                second.clone(),
                                speed,
                            );

                            if ticket.already_sent(&sent) {
                                continue;
                            }

                            if let Some(sender) = dispatchers.get(&ticket.road)
                            {
                                if let Ok(_) = sender
                                    .send(Message::Encoded(
                                        "C->T".into(),
                                        ticket.encode(),
                                    ))
                                    .await
                                {
                                    println!("Sent ticket");
                                    ticket.sent(&mut sent);
                                }
                            } else {
                                outbox.push(ticket);
                            }
                        }
                    }
                    inner_vec.retain(|t| t.keep);
                } else {
                    timestamps.insert(key, vec![timestamp]);
                }
            }
            Registration::NewDispatcher(roads, sender) => {
                // get relevant pending tickets, if any
                let pending =
                    outbox.iter_mut().filter(|t| roads.contains(&t.road));

                // try to send them
                for ticket in pending {
                    if ticket.already_sent(&sent) {
                        ticket.pending = false;
                        continue;
                    }

                    if let Ok(_) = sender
                        .send(Message::Encoded("D->T".into(), ticket.encode()))
                        .await
                    {
                        ticket.sent(&mut sent);
                    }
                }

                outbox.retain(|t| t.pending);

                // finally, we register the dispatcher
                for road in roads {
                    dispatchers.insert(road, sender.clone());
                }
            }
        }
    }
}


pub async fn runserver_async() {
    let addr = "0.0.0.0:8838";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", addr);

    let (tm_sender, mut tm_receiver): (RSender, RReceiver) = channel(200);

    tokio::spawn(async move { ticketmaster(tm_receiver).await });

    let mut counter = 0;

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection!");
        let sender_clone = tm_sender.clone();

        counter += 1;

        let idx = counter.clone();

        tokio::spawn(async move {
            let mut client = Client::new(idx, socket, sender_clone);
            client.handle().await;
        });
    }
}
