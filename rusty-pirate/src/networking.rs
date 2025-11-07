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
    let _ = self.listen_tcp.await;
    let _ = self.send_tcp.await;
    let _ = self.listen_udp.await;
    let _ = self.send_udp.await;
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

    println!("listen_tcp receive");

    while let Ok(msg) = decode_bytes(&bytes_buffer) {
      println!("decoded message: {:?}", msg);
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

mod tests {
  use super::*;
  use rusty_protocol::rusty_protocol::{create_message, RPMessageType};
use tokio::net::TcpListener;
  fn dummy_msg_1() ->Message { 
    create_message(RPMessageType::Cdirectory, vec!["test1".to_string(), "test2".to_string()]) 
  }

  fn dummy_msg_2() -> Message {
    create_message(RPMessageType::Text, vec!["I".to_string(), "Am".to_string(), "Yoshi".to_string(), "King".to_string()])
  }

  async fn create_tcp_server() -> Result<TcpListener, Error> {
    Ok(TcpListener::bind("127.0.0.1:6379").await.unwrap())
  }

  async fn create_tcp_client() -> Result<(OwnedReadHalf, OwnedWriteHalf), Error> {
    println!("creating tcp client");
    let tcp = TcpStream::connect("127.0.0.1:6379").await?;
    println!("connected to tcp connection");
    Ok(tcp.into_split())
  }

  // tcp tests
  #[tokio::test]
  async fn tcp_basic_recv() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let listen_handle = tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    println!("accepted client");
    socket.write(&encode_message(dummy_msg_1())).await.unwrap();
    println!("sent message");

    if let Some(msg) = rx.recv().await {
      assert_eq!(msg, dummy_msg_1());
      println!("good message!")
    } else {
      panic!("failed to receive message!")
    }
  }

  #[test]
  fn tcp_continuous_multiple_recv() {

  }

  #[test]
  fn tcp_lagged_multiple_recv() {

  }

  #[test]
  fn tcp_buffers_at_max_recv() {

  }

  #[test]
  #[should_panic]
  fn tcp_recv_connection_fail() {

  }

  #[test]
  fn tcp_basic_send() {

  }

  #[test] 
  fn tcp_continuous_multiple_send() {

  }

  #[test]
  fn tcp_lagged_multiple_send() {

  }

  #[test]
  #[should_panic]
  fn tcp_send_fail_on_error() {

  }


  // udp tests
  #[test]
  fn udp_basic_recv() {

  }

  #[test]
  fn udp_continuous_multiple_recv() {

  }

  #[test]
  fn udp_lagged_multiple_recv() {

  }

  #[test]
  fn udp_buffers_at_max_recv() {

  }

  #[test]
  #[should_panic]
  fn udp_recv_connection_fail() {

  }

  #[test]
  fn udp_basic_send() {

  }

  #[test] 
  fn udp_continuous_multiple_send() {

  }

  #[test]
  fn udp_lagged_multiple_send() {

  }

  #[test]
  #[should_panic]
  fn udp_send_fail_on_error() {

  }
}