use std::io;

use crate::networking::{create_connection};
mod networking;

#[tokio::main]
async fn main() -> io::Result<()> {

    const TCP_SEND_ADDR : &str = "127.0.0.1:6379";
    const UDP_SEND_ADDR: &str = "127.0.0.1:6380";
    const UDP_RECV_ADDR: &str = "127.0.0.1:6381";
    
    let ( connection_handles, _tx_tcp, _tx_udp, _rx_user ) = create_connection(TCP_SEND_ADDR, UDP_RECV_ADDR, UDP_SEND_ADDR).await.expect("failed to connect to servers");
    connection_handles.await_handles().await;

    Ok(())

}
