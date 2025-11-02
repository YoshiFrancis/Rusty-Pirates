use std::fmt::Debug;

#[derive(Debug)]
pub enum RPMessageType {
  Text = 0,
  Fneed = 1,
  Freq = 2,
  Pdirectory = 3,
  Cdirectory = 4,
  Info = 10,
  Pnamereq = 5,
  Cinforeq = 6,
  // cabin requests
  Pinput = 7,
  Join = 8,
  Leave = 9,
  // client response
  Name = 11,
}

#[derive(Debug)]
pub struct Message {
  rp_type: RPMessageType,
  args: Vec<String>
}

#[derive(Debug)]
pub struct MessageErr {
  err : String
}

pub fn create_message(rp_type : RPMessageType, args : Vec<String>) -> Message {
  Message { rp_type, args }
}

pub fn message_to_bytes(message : Message) -> Vec<u8> {
  let fs : u8 = 34;
  let mut bytes : Vec<u8> = vec![];
  let rp_type = message.rp_type as u32;
  // using little endian
  bytes.extend_from_slice(&rp_type.to_le_bytes());
  bytes.push(fs);
  for arg in message.args {
    bytes.extend_from_slice(arg.as_bytes());
    bytes.push(fs);
  }
  bytes.push(fs);

  bytes
}

pub fn bytes_to_message(bytes : &[u8]) -> Result<Message, MessageErr> {
  let fs : u8 = 34;

  if bytes.len() < 6 {
    return Err(MessageErr { err: (String::from("Not enough bytes to get four byte message and end double FS")) })
  }

  let mut iter = bytes[0..bytes.len() - 2].split(|&byte| byte == fs).enumerate();
  let (_, bytes_first_four) = iter.next().expect("Failed to get first four bytes of message");
  let size = iter.clone().count();
  let rp_type_u32 = u32::from_le_bytes(bytes_first_four
    .try_into()
    .expect("Failed to convert first 4 bytes of message to u32"));

  if bytes[bytes.len() - 1] != fs && bytes[bytes.len() - 2] != fs {
    return Err(MessageErr { err: (String::from("Did not receive double FS at the end of message")) })
  }

  let mut args  = vec![];
  args.resize(size, String::from(""));
  while let Some((idx, bytes)) = iter.next() {
    args[idx-1] = String::from_utf8(
      bytes.try_into().expect("Could not convert &[u8] to Vec<u8>")
    ).expect("Could not convert Vec<u8> to String");
  };

  match u32_to_rptype(rp_type_u32) {
    Ok(rp_type) => Ok(Message { rp_type, args }),
    Err(error) => Err(error)
  }

}

fn u32_to_rptype(rp_type_id : u32) -> Result<RPMessageType, MessageErr> {
  match rp_type_id {
    0 => Ok(RPMessageType::Text),
    1 => Ok(RPMessageType::Fneed),
    2 => Ok(RPMessageType::Freq),
    3 => Ok(RPMessageType::Pdirectory),
    4 => Ok(RPMessageType::Cdirectory),
    5 => Ok(RPMessageType::Pnamereq),
    6 => Ok(RPMessageType::Cinforeq),
    7 => Ok(RPMessageType::Pinput),
    8 => Ok(RPMessageType::Join),
    9 => Ok(RPMessageType::Leave),
    10 => Ok(RPMessageType::Info),
    11 => Ok(RPMessageType::Name),
    _ => Err(MessageErr {err: 
      format!("failed to parse id to Rusty Protocol message type: {}", rp_type_id)
      })
  }
}

impl TryFrom<u8> for RPMessageType {
    type Error = ();
    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            0 => Ok(Self::Text),
            1 => Ok(Self::Fneed),
            2 => Ok(Self::Freq),
            3 => Ok(Self::Pdirectory),
            4 => Ok(Self::Cdirectory),
            5 => Ok(Self::Pnamereq),
            6 => Ok(Self::Cinforeq),
            7 => Ok(Self::Pinput),
            8 => Ok(Self::Join),
            9 => Ok(Self::Leave),
            10 => Ok(Self::Info),
            _ => Err(()),
        }
    }
}