use std::fmt::Error;

use crate::rusty_protocol::{self, Message};

struct ClientCtx {

  // TCP stream
  // UDP stream 
}

pub fn connect(address : &str) -> Option<Error> {
  todo!("simple_pirate/connect TODO");
  None
}

pub fn disconnect() -> Option<Error> {
  todo!("simple_pirate/disconnect TODO");
  None
}

// should be async
pub fn listen_tcp(ctx : &ClientCtx) -> rusty_protocol::Message {
  todo!("simple_pirate/listen_tcp TODO");
}

// should be async
pub fn listen_udp(ctx : &ClientCtx) -> rusty_protocol::Message {
  todo!("simple_pirate/listen_udp TODO");
}

// should be async
pub fn get_input() -> rusty_protocol::Message {
  todo!("simple_pirate/get_input TODO");
  rusty_protocol::create_message(rusty_protocol::RPMessageType::Text, vec![])
}

// should be async
pub fn send_message_tcp() -> Option<Error> {
  todo!("simple_pirate/send_message_tcp TODO");
  None
}

// should be async
pub fn send_message_udp() -> Option<Error> {
  todo!("simple_pirate/send_message_udp TODO");
  None
}

