use std::io::Error;
use std::sync::mpsc::Sender;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc::{self, Receiver};

use rusty_protocol::rusty_protocol::{Message, RPMessageType, create_message, encode_message, decode_bytes};

pub struct PirateCtx {

  // TCP stream
  tcp: TcpStream,
  // UDP Socket 
  udp: UdpSocket,
}

pub async fn create_pirate_ctx(tcp_addr : &str, udp_addr_listen : &str, udp_addr_send : &str) -> Result<PirateCtx, Error> {
  let udp = UdpSocket::bind(udp_addr_listen).await?;
  udp.connect(udp_addr_send);
  let tcp = TcpStream::connect(tcp_addr).await.expect("Could not bind to tcp stream for pirate context creation");
  Ok(PirateCtx { tcp: tcp, udp: udp })
}

// should be async
pub async fn listen_tcp(ctx : &mut PirateCtx, tx : Sender<Message>) -> Option<String>{
  let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
  loop {
    if 0 == ctx.tcp.read_buf(&mut bytes_buffer).await.unwrap() {
      if bytes_buffer.is_empty() {
        return None;
      } else {
        return Some("peer reset connection -> unfinished bytes in buffer".into());
      }
    }
    while let Ok(msg) = decode_bytes(&bytes_buffer) {
      tx.send(msg);
    }
  }
}

// should be async
pub async fn send_message_tcp(ctx : &mut PirateCtx, mut rx : Receiver<Message>) -> Option<String> {
  while let Some(msg) = rx.recv().await {
    ctx.tcp.write(&encode_message(msg)).await.expect("Failed to send to user");
  }
  None
}

// should be async
pub async fn listen_udp(ctx : &PirateCtx, tx: Sender<Message>) -> Option<String> {
  let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
  
  loop {
    if 0 == ctx.udp.recv(&mut bytes_buffer).await.unwrap() {
      if bytes_buffer.is_empty() {
        return None;
      } else {
        return Some("peer reset connection -> unfinished bytes in buffer".into());
      }
    }
    while let Ok(msg) = decode_bytes(&bytes_buffer) {
      tx.send(msg);
    }
  }
}

pub async fn send_message_udp(ctx: &mut PirateCtx, mut rx: Receiver<Message>) -> Option<Error> {
  while let Some(msg) = rx.recv().await {
    ctx.udp.send(&encode_message(msg));
  }
  None
}

