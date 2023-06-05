use std::time::Duration;
use chrono::{NaiveDateTime, NaiveDate};

use anyhow::{bail, Error, Result};
use tokio::io::{AsyncReadExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time;

pub mod constants {
    pub const ERROR: u8 = 16; // 0x10
    pub const PLATE: u8 = 32; // 0x20
    pub const TICKET: u8 = 33; // 0x21
    pub const WANTHEARTBEAT: u8 = 64; // 0x40
    pub const HEARTBEAT: u8 = 65; // 0x41

    pub const CAMERA: u8 = 128; // 0x80
    pub const DISPATCHER: u8 = 129; // 0x81
}


#[derive(Debug)]
pub enum Message {
    Encoded(String, Vec<u8>),
    Terminate,
}

pub type XReader = BufReader<ReadHalf<TcpStream>>;
pub type XSender = Sender<Message>;
pub type XReceiver = Receiver<Message>;

#[derive(Debug, Clone)]
pub struct Camera {
    pub road: u16,
    // mile is also the position of the camera
    pub mile: u16,
    pub limit: u16,
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Timestamp {
    pub road: u16,
    pub mile: u16,
    pub limit: u16,
    pub timestamp: u32,
    pub keep: bool,
}


impl Timestamp {
    pub fn new(cam: &Camera, timestamp: u32) -> Self {
        Self {
            road: cam.road.clone(),
            mile: cam.mile.clone(),
            limit: cam.limit.clone(),
            timestamp: timestamp,
            keep: true,
        }
    }
}


#[derive(Debug)]
pub struct Ticket {
    pub plate: String,
    pub road: u16,
    mile1: u16,
    pub timestamp1: u32,
    mile2: u16,
    timestamp2: u32,
    speed: u16,
    pub pending: bool,
}

impl Ticket {
    pub fn new(
        plate: String,
        ts1: Timestamp,
        ts2: Timestamp,
        speed: u16,
    ) -> Self {
        Self {
            plate: plate,
            road: ts1.road,
            mile1: ts1.mile,
            timestamp1: ts1.timestamp,
            mile2: ts2.mile,
            timestamp2: ts2.timestamp,
            speed: speed,
            pending: true,
        }
    }

    pub fn date_keys(&self) -> [(String, NaiveDate); 2] {
        let key = (
            self.plate.clone(),
            NaiveDateTime::from_timestamp_opt(self.timestamp1.into(), 0)
                .unwrap()
                .date(),
        );
        let key2 = (
            self.plate.clone(),
            NaiveDateTime::from_timestamp_opt(self.timestamp2.into(), 0)
                .unwrap()
                .date(),
        );
        return [key, key2];
    }

    pub fn already_sent(&self, sent: &Vec<(String, NaiveDate)>) -> bool {
        for key in self.date_keys() {
            if sent.contains(&key) {
                return true;
            }
        }

        false
    }

    pub fn sent(&mut self, sent: &mut Vec<(String, NaiveDate)>) {
        self.pending = false;
        for key in self.date_keys() {
            if !sent.contains(&key) {
                sent.push(key);
            }
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out: Vec<u8> = vec![constants::TICKET];

        let plate_b = self.plate.clone().into_bytes();
        out.push(plate_b.len() as u8);
        out.extend(plate_b);
        out.extend(self.road.to_be_bytes());
        out.extend(self.mile1.to_be_bytes());
        out.extend(self.timestamp1.to_be_bytes());
        out.extend(self.mile2.to_be_bytes());
        out.extend(self.timestamp2.to_be_bytes());
        out.extend(self.speed.to_be_bytes());
        out
    }
}


pub fn encode_error() -> Message {
    let mut out: Vec<u8> = vec![constants::ERROR];
    let msg = "Error!".to_string().into_bytes();
    out.push(msg.len() as u8);
    out.extend(msg);
    Message::Encoded("Err".into(), out)
}
