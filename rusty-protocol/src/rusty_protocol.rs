use std::fmt::Debug;

#[derive(Debug, PartialEq)]
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
    return Err(MessageErr { err: (String::from("Not enough bytes")) })
  }

  if bytes[bytes.len() - 1] != fs || bytes[bytes.len() - 2] != fs {
    return Err(MessageErr { err: (String::from("No ending double FS")) })
  }

  let mut iter = bytes[0..bytes.len() - 2].split(|&byte| byte == fs).enumerate();
  let (_, bytes_first_four) = iter.next().expect("Failed to get first four bytes of message");
  let size = iter.clone().count();
  let rp_type_u32 = u32::from_le_bytes(bytes_first_four
    .try_into()
    .expect("Failed to convert first 4 bytes of message to u32"));

  

  let mut args  = vec![];
  args.resize(size, String::from(""));
  while let Some((idx, bytes)) = iter.next() {
    args[idx-1] = String::from_utf8(
      bytes.try_into().expect("Could not convert &[u8] to Vec<u8>")
    ).expect("Could not convert Vec<u8> to String");
  };

  match RPMessageType::try_from(rp_type_u32) {
    Ok(rp_type) => Ok(Message { rp_type, args }),
    Err(error) => Err(error)
  }

}

impl TryFrom<u32> for RPMessageType {
  type Error = MessageErr;
  fn try_from(n: u32) -> Result<Self, MessageErr> {
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
          _ => Err(MessageErr{err: String::from("failed")}),
      }
  }
}

#[cfg(test)]
mod tests {
    use std::io::Bytes;

    use super::*;

    const FS : u8 = 34;

    #[test]
    fn parse_message_ok() {
      let expected_message = Message {
        rp_type: RPMessageType::Cdirectory,
        args: vec![String::from("testing1"), String::from("testing2")]
      };

      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;

      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      message_bytes.push(FS);
      message_bytes.push(FS);

      let actual_message = bytes_to_message(&message_bytes).expect("Failed to parse ok message");
      assert_eq!(actual_message.rp_type, expected_message.rp_type);
      assert_eq!(actual_message.args, expected_message.args);

    }

    #[test]
    #[should_panic]
    fn parse_message_fail_bad_fs() {
      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      message_bytes.push(FS);

      match bytes_to_message(&message_bytes) {
        Err(_) => panic!("Correct panic"),
        Ok(_) => println!("Passed with no double fs at end: {:#?}", message_bytes)
      }
    }

    #[test]
    #[should_panic]
    fn parse_message_fail_rp_type() {
      let cdirectory_type : u32 = 100;
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      message_bytes.push(FS);
      message_bytes.push(FS);

      bytes_to_message(&message_bytes).unwrap();
    }

    #[test]
    fn decode_message_ok() {
      
    }

    #[test]
    fn decode_message_fail() {

    }

    #[test] 
    fn message_in_and_out() {

    }

}