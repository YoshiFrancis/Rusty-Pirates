use crate::rusty_protocol::{bytes_to_message, message_to_bytes};

mod rusty_protocol;


fn main() {
    println!("Hello, world!");
    let rp_type = rusty_protocol::RPMessageType::Fneed;
    let args = vec![String::from("Hello"), String::from("World")];
    let message = rusty_protocol::create_message(rp_type, args);
    println!("Message: {:#?}", message);
    let bytes_message = message_to_bytes(message);
    println!("Bytes: {:#?}", bytes_message);
    let message = bytes_to_message(&bytes_message).expect("Failed to remake message from btyes");
    println!("Remade Message: {:#?}", message);

}
