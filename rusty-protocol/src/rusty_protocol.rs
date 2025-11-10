use std::{cmp::max, fmt::Debug};

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ParseResult {
  NotEnoughBytes,
  IncompleteEnd,
  Invalid(usize), // invalid beginning or end
  Complete(usize)
}

#[derive(Debug, Clone, PartialEq)]
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

pub fn encode_message(message : Message) -> Vec<u8> {
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

pub fn check(bytes : &[u8], starting_pos : usize) -> Option<ParseResult> {
  let fs : u8 = 34;

  if bytes.len() < 6 {
    return Some(ParseResult::NotEnoughBytes);
  }

  // its ugly,... I KNOW D:
  if starting_pos == 0 {
    let mut found = false;
    for i in 0..bytes.len() - 3 {
      let rp_type_u32 = u32::from_le_bytes(bytes[i..i+4].try_into().unwrap());
      if let Ok(_) = RPMessageType::try_from(rp_type_u32) {
        if i == 0 {
          found = true;
          break;
        } else {
          return Some(ParseResult::Invalid(i)); // found beginning of valid point
        }
      }
    }
    if !found {
      return Some(ParseResult::Invalid(bytes.len())); // found no valid starting byte message for the message
    }
  }

  let starting_pos = max(starting_pos, 5);

  for idx in starting_pos..bytes.len() {
    if bytes[idx-1] == fs && bytes[idx] == fs {
      return Some(ParseResult::Complete(idx+1));
    }
  }
  return Some(ParseResult::IncompleteEnd);
}

pub fn decode_bytes(bytes : &[u8]) -> Result<Message, MessageErr> {
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
    fn check_complete() {
      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      message_bytes.push(FS);
      message_bytes.push(FS);

      let original_length = message_bytes.len();

      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Complete(original_length));

      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      message_bytes.push(FS);
      message_bytes.push(FS);

      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Complete(original_length));
      assert_eq!(check(&message_bytes, original_length).unwrap(), ParseResult::Complete(message_bytes.len()));
    }

    #[test]
    fn check_invalid_beginning() {
      let invalid_type : u32 = 100;
      
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&invalid_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.push(FS);
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Invalid(6));
      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Invalid(6));
    }

    #[test]
    fn check_not_enough_bytes() {
      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::NotEnoughBytes);
      message_bytes.push(FS);
      message_bytes.push(FS);
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Complete(message_bytes.len()));
    }

    #[test]
    fn check_incomplete_end() {
      let cdirectory_type : u32 = RPMessageType::Cdirectory as u32;
      let mut message_bytes : Vec<u8> = vec![];
      message_bytes.extend_from_slice(&cdirectory_type.to_le_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      message_bytes.push(FS);
      message_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::IncompleteEnd);
      message_bytes.push(FS);
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::IncompleteEnd);
      message_bytes.push(FS);
      assert_eq!(check(&message_bytes, 0).unwrap(), ParseResult::Complete(message_bytes.len()));
    }

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

      let actual_message = decode_bytes(&message_bytes).expect("Failed to parse ok message");
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

      match decode_bytes(&message_bytes) {
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

      decode_bytes(&message_bytes).unwrap();
    }

    #[test]
    fn encode_message_ok() {
      let freq_type : u32 = RPMessageType::Freq as u32;
      let mut expected_bytes : Vec<u8> = vec![];
      expected_bytes.extend_from_slice(&freq_type.to_le_bytes());
      expected_bytes.push(FS);
      expected_bytes.extend_from_slice(&String::from("testing1").as_bytes());
      expected_bytes.push(FS);
      expected_bytes.extend_from_slice(&String::from("testing2").as_bytes());
      expected_bytes.push(FS);
      expected_bytes.push(FS);

      let given_message = Message {
        rp_type : RPMessageType::Freq, args : vec![String::from("testing1"), String::from("testing2")]
      };

      let actual_bytes = encode_message(given_message);
      assert_eq!(actual_bytes, expected_bytes);
      
    }

    #[test] 
    fn message_in_and_out() {
      let given_message = Message {
        rp_type : RPMessageType::Text, args : vec![String::from("testing1"), String::from("testing2")]
      };

      let encoded_message_bytes = encode_message(given_message.clone());
      let decoded_message = decode_bytes(&encoded_message_bytes).unwrap();

      assert_eq!(decoded_message, given_message);
    }

}