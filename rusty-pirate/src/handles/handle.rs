use rusty_protocol::rusty_protocol::{Message, create_message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender};


pub async fn handle_messages(
  tx_tcp : Sender<Message>, 
  tx_udp: Sender<Message>, 
  tx_loop: Sender<Message>,
  rx_user: &mut Receiver<Message>
) {

  while let Some(msg) = rx_user.recv().await {
    use rusty_protocol::rusty_protocol::RPMessageType;
    // handle message
    use crate::handles::text_handles;
    match msg.rp_type {
      RPMessageType::Text       => text_handles::handle_text(msg),
      RPMessageType::Cdirectory => text_handles::handle_cdirectory(msg),
      RPMessageType::Fneed      => text_handles::handle_fneed(msg),
      RPMessageType::Pdirectory => text_handles::handle_pdirectory(msg),
      RPMessageType::Info       => text_handles::handle_info(msg),
      RPMessageType::Pinput     => text_handles::handle_pinput(msg),
      RPMessageType::Pnamereq   => text_handles::handle_pnamereq(msg),
      _ => {
        println!("Receieved unhandled message type: {:?}", msg.rp_type);
      }
    }
  }

}