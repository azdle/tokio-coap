pub mod option;

#[derive(PartialEq, Eq, Debug)]
pub struct Message {
    pub version: u8,
    pub mtype: Mtype,
    pub code: Code,
    pub mid: u16,
    pub token: Vec<u8>,
    pub options: Vec<option::Option>,
    pub payload: Vec<u8>,
}

#[derive(PartialEq, Debug)]
pub enum Error {
    MessageFormat,
    InvalidToken,
    InvalidOptionNumber,
    UnrecognizedCriticalOption, // TODO: use
}

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mtype {
    Confirmable,
    NonConfirmable,
    Acknowledgement,
    Reset,
}

impl Mtype {
    pub fn from_u8(raw_mtype: u8) -> Mtype {
        match raw_mtype {
            0 => Mtype::Confirmable,
            1 => Mtype::NonConfirmable,
            2 => Mtype::Acknowledgement,
            3 => Mtype::Reset,
            _ => panic!("bad rawtype"),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match *self {
            Mtype::Confirmable => 0,
            Mtype::NonConfirmable => 1,
            Mtype::Acknowledgement => 2,
            Mtype::Reset => 3,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Code {
    Empty,
    Get,
    Post,
    Put,
    Delete,
    Created,
    Deleted,
    Valid,
    Changed,
    Content,
    BadRequest,
    Unauthorized,
    BadOption,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    PreconditionFailed,
    RequestEntityTooLarge,
    UnsupportedContentFormat,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    ProxyingNotSupported,
    Unknown(u8),
}

impl Code {
    pub fn from_u8(raw_code: u8) -> Code {
        match raw_code {
            0 => Code::Empty,
            1 => Code::Get,
            2 => Code::Post,
            3 => Code::Put,
            4 => Code::Delete,
            65 => Code::Created,
            66 => Code::Deleted,
            67 => Code::Valid,
            68 => Code::Changed,
            69 => Code::Content,
            128 => Code::BadRequest,
            129 => Code::Unauthorized,
            130 => Code::BadOption,
            131 => Code::Forbidden,
            132 => Code::NotFound,
            133 => Code::MethodNotAllowed,
            134 => Code::NotAcceptable,
            140 => Code::PreconditionFailed,
            141 => Code::RequestEntityTooLarge,
            142 => Code::UnsupportedContentFormat,
            160 => Code::InternalServerError,
            161 => Code::NotImplemented,
            162 => Code::BadGateway,
            163 => Code::ServiceUnavailable,
            164 => Code::GatewayTimeout,
            165 => Code::ProxyingNotSupported,
            _ => Code::Unknown(raw_code),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match *self {
            Code::Empty => Self::build(0, 00),
            Code::Get => Self::build(0, 01),
            Code::Post => Self::build(0, 02),
            Code::Put => Self::build(0, 03),
            Code::Delete => Self::build(0, 04),
            Code::Created => Self::build(2, 01),
            Code::Deleted => Self::build(2, 02),
            Code::Valid => Self::build(2, 03),
            Code::Changed => Self::build(2, 04),
            Code::Content => Self::build(2, 05),
            Code::BadRequest => Self::build(4, 00),
            Code::Unauthorized => Self::build(4, 01),
            Code::BadOption => Self::build(4, 02),
            Code::Forbidden => Self::build(4, 03),
            Code::NotFound => Self::build(4, 04),
            Code::MethodNotAllowed => Self::build(4, 05),
            Code::NotAcceptable => Self::build(4, 06),
            Code::PreconditionFailed => Self::build(4, 12),
            Code::RequestEntityTooLarge => Self::build(4, 13),
            Code::UnsupportedContentFormat => Self::build(4, 15),
            Code::InternalServerError => Self::build(5, 00),
            Code::NotImplemented => Self::build(5, 01),
            Code::BadGateway => Self::build(5, 02),
            Code::ServiceUnavailable => Self::build(5, 03),
            Code::GatewayTimeout => Self::build(5, 04),
            Code::ProxyingNotSupported => Self::build(5, 05),
            Code::Unknown(code) => code,
        }
    }

    #[inline(always)]
    fn build(class: u8, detail: u8) -> u8 {
        ((class & 0x07) << 5) | (detail & 0x1F)
    }

    pub fn class(&self) -> u8 {
        self.as_u8() >> 5
    }

    pub fn detail(&self) -> u8 {
        self.as_u8() & 0x1F
    }
}

impl Message {
    pub fn from_bytes(pkt: &[u8]) -> Result<Message, Error> {
        let mut i: usize;

        if pkt.len() < 4 {
            return Err(Error::MessageFormat);
        }

        let version = pkt[0] >> 6;
        let mtype = Mtype::from_u8((pkt[0] >> 4) & 0x03);
        let token_length = pkt[0] & 0x0F;
        let code = Code::from_u8(pkt[1]);
        let mid = ((pkt[2] as u16) << 8) | pkt[3] as u16;

        if pkt.len() < 4 + token_length as usize {
            return Err(Error::MessageFormat);
        }

        let mut token = vec![];
        for j in 0..token_length {
            token.push(pkt[4 + j as usize]);
        }

        i = 4 + token_length as usize;

        let mut options: Vec<option::Option> = vec![];
        let mut option_number_offset = 0u16;

        while i < pkt.len() {
            if pkt[i] == 0xFF {
                i += 1;
                break;
            }

            // Note: length errors for 13 & 14 will be caught in the check below.
            let delta = match pkt[i] >> 4 {
                d @ 0...12 => d as u16,
                13 => {
                    i += 1;
                    pkt[i] as u16 + 13
                }
                14 => {
                    i += 2;
                    (((pkt[i - 1] as u16) << 8) | pkt[i] as u16) + 269
                }
                15 => panic!("message format error"),
                _ => unreachable!(),
            };
            let length = match pkt[i] & 0x0F {
                d @ 0...12 => d as u16,
                13 => {
                    i += 1;
                    pkt[i] as u16 + 13
                }
                14 => {
                    i += 2;
                    ((pkt[i - 1] as u16) << 8) | pkt[i] as u16 + 269
                }
                15 => panic!("message format error"),
                _ => unreachable!(),
            };

            i += 1;

            let option_number = option_number_offset + delta;
            option_number_offset = option_number;

            if length >= 65000 {
                return Err(Error::MessageFormat);
            }

            if pkt.len() >= i + (length as usize) {
                options.push(option::Option::from_raw(option_number, &pkt[i..i+(length as usize)]));
            } else {
                return Err(Error::MessageFormat);
            }

            i += length as usize;
        }

        let payload = if i < pkt.len() {
            pkt[i..].to_vec()
        } else {
            vec![]
        };

        Ok(Message {
            version: version,
            mtype: mtype,
            code: code,
            mid: mid,
            token: token,
            options: options,
            payload: payload,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        if self.token.len() > 8 {
            return Err(Error::MessageFormat);
        }

        // estimate packet size
        let mut est_pkt_size: usize = 4 + self.token.len() + 1 + 1 + self.payload.len();

        for option in &self.options {
            est_pkt_size += 2 + option.value_len();

            if option.number() >= 65000 {
                return Err(Error::MessageFormat);
            }
        }

        let mut pkt = Vec::with_capacity(est_pkt_size);

        pkt.push((self.version << 6) | self.mtype.as_u8() << 4 | self.token.len() as u8);
        pkt.push(self.code.as_u8());
        pkt.push((self.mid >> 8) as u8);
        pkt.push((self.mid & 0xFF) as u8);

        for byte in &self.token {
            pkt.push(*byte)
        }

        let mut last_option_number = 0;

        for option in &self.options {
            pkt.extend(option.build_header(&mut last_option_number));
            pkt.extend(option.value_to_bytes().iter());
        }

        if self.payload.len() > 0 {
            pkt.push(0xFF);
            pkt.extend(&self.payload);
        }

        Ok(pkt)
    }
}


#[test]
fn test_msg_parse_empty() {
    let ref_bin = [64, 0, 0, 0];

    let msg = Message::from_bytes(&ref_bin).unwrap();

    assert!(msg.version == 1);
    assert!(msg.mtype == Mtype::Confirmable);
    assert!(msg.code == Code::Empty);
    assert!(msg.code.class() == 0);
    assert!(msg.code.detail() == 0);
    assert!(msg.mid == 0);
    assert!(msg.token.len() == 0);
    assert!(msg.options.len() == 0);
    assert!(msg.payload.len() == 0);
}

#[test]
fn test_msg_serialize_empty() {
    let ref_bin = [64, 0, 0, 0];
    let msg = Message {
        version: 1,
        mtype: Mtype::Confirmable,
        code: Code::Empty,
        mid: 0,
        token: vec![],
        options: vec![],
        payload: vec![],
    };

    let test_bin = msg.to_bytes().unwrap();

    assert!(test_bin == ref_bin);
}

#[test]
fn test_msg_parse_empty_con_with_token() {
    let ref_bin = [66, 0, 0, 0, 37, 42];

    let msg = Message::from_bytes(&ref_bin).unwrap();

    assert!(msg.version == 1);
    assert!(msg.mtype == Mtype::Confirmable);
    assert!(msg.code == Code::Empty);
    assert!(msg.code.class() == 0);
    assert!(msg.code.detail() == 0);
    assert!(msg.mid == 0);
    assert!(msg.token == [37, 42]);
    assert!(msg.options.len() == 0);
    assert!(msg.payload.len() == 0);
}

#[test]
fn test_msg_parse_get_con() {
    let ref_bin = [0x41, 0x01, 0x00, 0x37, 0x99, 0xFF, 0x01, 0x02];

    let msg = Message::from_bytes(&ref_bin).unwrap();

    assert!(msg.version == 1);
    assert!(msg.mtype == Mtype::Confirmable);
    assert!(msg.code == Code::Get);
    assert!(msg.code.class() == 0);
    assert!(msg.code.detail() == 1);
    assert!(msg.mid == 0x37);
    assert!(msg.token == [0x99]);
    assert!(msg.options.len() == 0);
    assert!(msg.payload == [0x01, 0x02]);
}

#[test]
fn test_msg_parse_get_con_with_opts() {
    let ref_bin = [0x40, 0x02, 0x00, 0x37, 0xb2, 0x31, 0x61, 0x04, 0x74, 0x65, 0x6d, 0x70, 0x4d,
                   0x1b, 0x61, 0x33, 0x32, 0x63, 0x38, 0x35, 0x62, 0x61, 0x39, 0x64, 0x64, 0x61,
                   0x34, 0x35, 0x38, 0x32, 0x33, 0x62, 0x65, 0x34, 0x31, 0x36, 0x32, 0x34, 0x36,
                   0x63, 0x66, 0x38, 0x62, 0x34, 0x33, 0x33, 0x62, 0x61, 0x61, 0x30, 0x36, 0x38,
                   0x64, 0x37, 0xFF, 0x39, 0x39];

    let msg = Message::from_bytes(&ref_bin).unwrap();

    assert!(msg.version == 1);
    assert!(msg.mtype == Mtype::Confirmable);
    assert!(msg.code == Code::Post);
    assert!(msg.code.class() == 0);
    assert!(msg.code.detail() == 2);
    assert!(msg.mid == 0x0037);
    assert!(msg.token.len() == 0);
    assert!(msg.options ==
            [option::Option::UriPath("1a".to_string()),
             option::Option::UriPath("temp".to_string()),
             option::Option::UriQuery("a32c85ba9dda45823be416246cf8b433baa068d7".to_string())]);
    assert!(msg.payload == [0x39, 0x39]);
}

#[test]
fn test_msg_encode_get_con_with_opts() {
    let ref_bin = [0x40, 0x02, 0x00, 0x37, 0xb2, 0x31, 0x61, 0x04, 0x74, 0x65, 0x6d, 0x70, 0x4d,
                   0x1b, 0x61, 0x33, 0x32, 0x63, 0x38, 0x35, 0x62, 0x61, 0x39, 0x64, 0x64, 0x61,
                   0x34, 0x35, 0x38, 0x32, 0x33, 0x62, 0x65, 0x34, 0x31, 0x36, 0x32, 0x34, 0x36,
                   0x63, 0x66, 0x38, 0x62, 0x34, 0x33, 0x33, 0x62, 0x61, 0x61, 0x30, 0x36, 0x38,
                   0x64, 0x37, 0xFF, 0x39, 0x39];
    let msg = Message {
        version: 1,
        mtype: Mtype::Confirmable,
        code: Code::Post,
        mid: 0x0037,
        token: vec![],
        options: vec![option::Option::UriPath("1a".to_string()),
                      option::Option::UriPath("temp".to_string()),
                      option::Option::UriQuery("a32c85ba9dda45823be416246cf8b433baa068d7"
                          .to_string())],
        payload: vec![0x39, 0x39],
    };

    let test_bin = msg.to_bytes().unwrap();

    assert!(test_bin.len() == ref_bin.len());

    for i in 0..ref_bin.len() {
        assert_eq!(test_bin[i], ref_bin[i]);
    }
}
