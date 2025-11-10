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
    println!("received message to send on tcp");
    let n = wr.write(&encode_message(msg)).await.expect("Failed to send to ship via tcp");
    println!("sent {n} bytes on tcp");
    wr.flush().await.expect("could not flush tcp");
    println!("done flushing");
  }
  println!("done send_tcp");
  None
}

// should be async
async fn listen_udp(udp: Arc<UdpSocket>, tx: Sender<Message>) -> Option<String> {
  println!("listening udp");

  let mut bytes_buffer = [0u8; 2048]; // or reuse outside loop if needed
  let n = udp.recv(&mut bytes_buffer).await.expect("Failed to read udp");

  println!("received udp datagram ({} bytes)", n);
  let data = &bytes_buffer[..n];

  match decode_bytes(data) {
      Ok(msg) => {
          tx.send(msg).await.expect("failed to send udp received message back on tx");
      }
      _ => {
          eprintln!("failed to decode udp message: {:?}", data);
      }
  };

  None
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
  use tokio::{net::TcpListener, time::{sleep, Duration}};
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
    let tcp = TcpStream::connect("127.0.0.1:6379").await?;
    Ok(tcp.into_split())
  }

  async fn create_udp_server() -> Result<UdpSocket, Error> {
    Ok(UdpSocket::bind("127.0.0.1:6380").await?)
  }

  async fn create_udp_client() -> Result<UdpSocket, Error> {
    Ok(UdpSocket::bind("127.0.0.1:6381").await?)
  }

  async fn get_next_message(rx: &mut Receiver<Message>) -> Option<Message> {
    rx.recv().await
  }

  // tcp tests
  #[tokio::test]
  async fn tcp_basic_recv() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    socket.write(&encode_message(dummy_msg_1())).await.unwrap();

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
  }

  #[tokio::test]
  async fn tcp_continuous_multiple_recv() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    socket.write(&encode_message(dummy_msg_1())).await.unwrap();
    socket.write(&encode_message(dummy_msg_2())).await.unwrap();
    socket.write(&encode_message(dummy_msg_1())).await.unwrap();
    socket.write(&encode_message(dummy_msg_2())).await.unwrap();
    socket.write(&encode_message(dummy_msg_1())).await.unwrap();

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
  }

  #[tokio::test]
  async fn tcp_lagged_multiple_recv() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    tokio::spawn(async move {
      socket.write(&encode_message(dummy_msg_1())).await.unwrap();
      sleep(Duration::from_millis(100)).await;
      socket.write(&encode_message(dummy_msg_2())).await.unwrap();
      sleep(Duration::from_millis(100)).await;
      socket.write(&encode_message(dummy_msg_1())).await.unwrap();
      sleep(Duration::from_millis(100)).await;
      socket.write(&encode_message(dummy_msg_2())).await.unwrap();
      sleep(Duration::from_millis(100)).await;
      socket.write(&encode_message(dummy_msg_1())).await.unwrap();
    });

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
  }

  #[tokio::test]
  async fn tcp_buffers_at_max_recv() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    for i in 0..33 {
      if i % 2 == 0 {
        socket.write(&encode_message(dummy_msg_1())).await.unwrap();
      } else {
        socket.write(&encode_message(dummy_msg_2())).await.unwrap();
      }
    }

    sleep(Duration::from_secs(1)).await; // allow for the rx buffer to fill
    assert_eq!(rx.capacity(), 32);
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    // ensure that the tcp still has messagers to buffer in!
    assert_eq!(rx.capacity(), 32);
  }

  #[tokio::test]
  #[should_panic]
  async fn tcp_recv_connection_fail() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, mut _rx) = mpsc::channel(32);

    let (mut client_rd, _client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let handle = tokio::spawn( async move { 
      listen_tcp(&mut client_rd, tx).await;
    });

    let (mut socket, _) = listener.accept().await.unwrap();
    socket.write(&encode_message(dummy_msg_2())[..10]).await.unwrap();
    socket.shutdown().await.unwrap();
    handle.await.unwrap();
  }

  #[tokio::test]
  async fn tcp_basic_send() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, rx) = mpsc::channel(32);

    let (_client_rd, mut client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let (mut socket, _) = listener.accept().await.unwrap();

    tokio::spawn( async move { 
      send_tcp(&mut client_wr, rx).await.unwrap();
    });


    let _ = tx.send(dummy_msg_1()).await;
    let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
    let n = socket.read_buf(&mut bytes_buffer).await.unwrap();
    assert_eq!(n, 18);
    assert_eq!(decode_bytes(&bytes_buffer).unwrap(), dummy_msg_1());
  }

  #[tokio::test] 
  async fn tcp_continuous_multiple_send() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, rx) = mpsc::channel(32);

    let (_client_rd, mut client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let (mut socket, _) = listener.accept().await.unwrap();

    tokio::spawn( async move { 
      send_tcp(&mut client_wr, rx).await.unwrap();
    });

    let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
    for i in 0..5 {
      if i % 2 == 0 {
        let _ = tx.send(dummy_msg_1()).await;
        let n = socket.read_buf(&mut bytes_buffer).await.unwrap();
        assert_eq!(n, 18);
        assert_eq!(decode_bytes(&bytes_buffer).unwrap(), dummy_msg_1());
      } else {
        let _ = tx.send(dummy_msg_2()).await;
        let n = socket.read_buf(&mut bytes_buffer).await.unwrap();
        assert_eq!(n, 22);
        assert_eq!(decode_bytes(&bytes_buffer).unwrap(), dummy_msg_2());
      }
      bytes_buffer.clear();
    }
  }

  #[tokio::test]
  async fn tcp_lagged_multiple_send() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, rx) = mpsc::channel(32);

    let (_client_rd, mut client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let (mut socket, _) = listener.accept().await.unwrap();

    tokio::spawn( async move { 
      send_tcp(&mut client_wr, rx).await.unwrap();
    });

    let mut bytes_buffer: BytesMut = BytesMut::with_capacity(4096);
    for i in 0..5 {
      if i % 2 == 0 {
        let _ = tx.send(dummy_msg_1()).await;
        let n = socket.read_buf(&mut bytes_buffer).await.unwrap();
        assert_eq!(n, 18);
        assert_eq!(decode_bytes(&bytes_buffer).unwrap(), dummy_msg_1());
      } else {
        let _ = tx.send(dummy_msg_2()).await;
        let n = socket.read_buf(&mut bytes_buffer).await.unwrap();
        assert_eq!(n, 22);
        assert_eq!(decode_bytes(&bytes_buffer).unwrap(), dummy_msg_2());
      }
      bytes_buffer.clear();
      let _ = sleep(Duration::from_millis(50)).await;
    }
  }

  #[tokio::test]
  #[should_panic]
  async fn tcp_send_fail_on_error() {
    let listener = create_tcp_server().await.unwrap();
    let (tx, rx) = mpsc::channel(32);

    let (_client_rd, mut client_wr) = create_tcp_client().await.expect("Failed to make tcp client");
    let (socket, _) = listener.accept().await.unwrap();

    drop(socket);
    tx.send(dummy_msg_1()).await.unwrap();
    tx.send(dummy_msg_1()).await.unwrap(); // does not immediately see the dropped socket, so send again
                                           // analogous to my future pinging to know liveness
    let _ = send_tcp(&mut client_wr, rx).await.unwrap();
  }
  

  // udp tests
  #[tokio::test]
  async fn udp_basic_recv() {

    let server = create_udp_server().await.unwrap();
    let client = Arc::new(create_udp_client().await.unwrap());
    let (tx, mut rx) = mpsc::channel(32);

    let n = server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
    assert_eq!(n, 18);
    tokio::spawn(async move { 
      listen_udp(client, tx).await;
    });

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    
  }

  #[tokio::test]
  async fn udp_continuous_multiple_recv() {
    let server = create_udp_server().await.unwrap();
    let client = Arc::new(create_udp_client().await.unwrap());
    let (tx, mut rx) = mpsc::channel(32);

    let n = server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
    assert_eq!(n, 18);
    tokio::spawn(async move { 
      listen_udp(client, tx).await;
    });

    server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
    server.send_to(&encode_message(dummy_msg_2()), "127.0.0.1:6381").await.unwrap();
    server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
    server.send_to(&encode_message(dummy_msg_2()), "127.0.0.1:6381").await.unwrap();

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());

  }

  #[tokio::test]
  async fn udp_lagged_multiple_recv() {

    let server = create_udp_server().await.unwrap();
    let client = Arc::new(create_udp_client().await.unwrap());
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move { 
      listen_udp(client, tx).await;
    });

    tokio::spawn(async move {
      let n= server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
      assert_eq!(n, 18);
      sleep(Duration::from_millis(50)).await;
      server.send_to(&encode_message(dummy_msg_2()), "127.0.0.1:6381").await.unwrap();
      sleep(Duration::from_millis(60)).await;
      server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
      sleep(Duration::from_millis(75)).await;
      server.send_to(&encode_message(dummy_msg_2()), "127.0.0.1:6381").await.unwrap();
    });
    

    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_2());
  }

  #[tokio::test]
  async fn udp_buffers_at_max_recv() {
    let server = create_udp_server().await.unwrap();
    let client = Arc::new(create_udp_client().await.unwrap());
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move { 
      listen_udp(client, tx).await;
    });

    tokio::spawn(async move {
      for _ in 0..33 {
        let n= server.send_to(&encode_message(dummy_msg_1()), "127.0.0.1:6381").await.unwrap();
        assert_eq!(n, 18);
      }
    });

    assert_eq!(rx.capacity(), 32);
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(rx.capacity(), 32);
    assert_eq!(get_next_message(&mut rx).await.unwrap(), dummy_msg_1());
    assert_eq!(rx.capacity(), 31);
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