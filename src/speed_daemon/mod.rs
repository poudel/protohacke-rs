use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

mod message;
mod server;

use server::runserver_async;

pub fn runserver() {
    let rt = Runtime::new().unwrap();
    rt.block_on(runserver_async());
}
