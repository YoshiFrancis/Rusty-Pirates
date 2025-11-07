use std::io::Error;
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::Sender;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc::{self, Receiver};

use rusty_protocol::rusty_protocol::{Message, encode_message, decode_bytes};
use tokio::task::JoinHandle;

pub struct ConnectionHandles {
  listen_tcp: JoinHandle<()>,
  send_tcp: JoinHandle<()>,
  listen_udp: JoinHandle<()>,
  send_udp: JoinHandle<()>
}

impl ConnectionHandles {
  
  fn new(
    listen_tcp: JoinHandle<()>,
    send_tcp: JoinHandle<()>,
    listen_udp: JoinHandle<()>,
    send_udp: JoinHandle<()>
  ) -> ConnectionHandles {
    ConnectionHandles { 
      listen_tcp, send_tcp, listen_udp, send_udp
    }
  }

  pub async fn await_handles(self) {
    self.listen_tcp.await;
    self.send_tcp.await;
    self.listen_udp.await;
    self.send_udp.await;
  }
}

pub async fn create_connection(tcp_addr: &str, udp_addr_listen: &str, udp_addr_send : &str) -> Result<(ConnectionHandles, Sender<Message>, Sender<Message>, Receiver<Message>), Error> {
  let tcp = TcpStream::connect(tcp_addr).await.expect("Could not bind to tcp stream for pirate context creation");
  let read_udp = Arc::new(UdpSocket::bind(udp_addr_listen).await?);
  read_udp.connect(udp_addr_send).await.expect("udp failed to connect to ship udp port");
  let s_udp = read_udp.clone();

  // listeners to user, receiver for user
  let (tx_tcp_user, rx_user) = mpsc::channel(64);
  let tx_udp_user = tx_tcp_user.clone();
  // user to tcp, receiver for tcp
  let (tx_tcp, rx_tcp) = mpsc::channel(32);
  // user to udp, receiver for udp
  let (tx_udp, rx_udp) = mpsc::channel(32);

  let (mut tcp_rd, mut tcp_wr) = tcp.into_split();

  let listen_tcp_handle = tokio::spawn(async move {
        listen_tcp(&mut tcp_rd, tx_tcp_user).await.expect("listen tcp failed");
    });

    let listen_udp_handle = tokio::spawn(async move {
      listen_udp(read_udp, tx_udp_user).await.expect("listen udp failed");
    });

    let send_tcp_handle = tokio::spawn(async move {
      send_tcp(&mut tcp_wr, rx_tcp).await.expect("send tcp failed");
    });

    let send_udp_handle = tokio::spawn(async move {
      send_udp(s_udp, rx_udp).await.expect("send udp failed");
    });
  
  Ok((
    ConnectionHandles::new(listen_tcp_handle, send_tcp_handle, listen_udp_handle, send_udp_handle),
    tx_tcp, tx_udp, rx_user
  ))
}

async fn listen_tcp(rd : &mut OwnedReadHalf, tx: Sender<Message>) -> Option<String>{
  let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
  loop {
    if 0 == rd.read_buf(&mut bytes_buffer).await.unwrap() {
      if bytes_buffer.is_empty() {
        return None;
      } else {
        return Some("peer reset connection -> unfinished bytes in buffer".into());
      }
    }
    while let Ok(msg) = decode_bytes(&bytes_buffer) {
      tx.send(msg).await.expect("failed to send received tcp message bakc on tx");
    }
  }
}

// should be async
async fn send_tcp(wr: &mut OwnedWriteHalf, mut rx: Receiver<Message>) -> Option<String> {
  while let Some(msg) = rx.recv().await {
    wr.write(&encode_message(msg)).await.expect("Failed to send to ship via tcp");
  }
  None
}

// should be async
async fn listen_udp(udp: Arc<UdpSocket>, tx: Sender<Message>) -> Option<String> {
  let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
  
  loop {
    if 0 == udp.recv(&mut bytes_buffer).await.unwrap() {
      if bytes_buffer.is_empty() {
        return None;
      } else {
        return Some("peer reset connection -> unfinished bytes in buffer".into());
      }
    }
    while let Ok(msg) = decode_bytes(&bytes_buffer) {
      tx.send(msg).await.expect("failed to send udp received message back on tx");
    }
  }
}

async fn send_udp(udp: Arc<UdpSocket>, mut rx: Receiver<Message>) -> Result<(), Error> {
  while let Some(msg) = rx.recv().await {
    udp.send(&encode_message(msg)).await.expect("failed to send udp message to ship");
  }
  Ok(())
}