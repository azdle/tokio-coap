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

pub mod option {
    #[derive(PartialEq, Eq, Debug)]
    pub enum Option {
        IfMatch(Vec<u8>),
        UriHost(String),
        ETag(Vec<u8>),
        IfNoneMatch,
        Observe(u32),
        UriPort(u16),
        LocationPath(String),
        UriPath(String),
        ContentFormat(u16),
        MaxAge(u32),
        UriQuery(String),
        Accept(u16),
        LocationQuery(String),
        ProxyUri(String),
        ProxyScheme(String),
        Size1(u32),
        NoResponse(u8),
        Unknown((u16, Vec<u8>)),
    }

    impl Option {
        pub fn build_header(&self, last_option_number: &mut u16) -> Vec<u8> {
            let mut header = vec![0u8];

            if self.number() < *last_option_number {
                panic!("bad order");
            }

            let delta = self.number() - *last_option_number;
            let base_delta = match delta {
                0...12 => delta,
                13...268 => {
                    header.push((delta - 13) as u8);
                    13
                }
                269...64999 => {
                    header.push(((delta - 269) >> 8) as u8);
                    header.push((delta - 269) as u8);
                    14
                }
                _ => unreachable!(),
            } as u8;
            let length = self.value_len();
            let base_length = match length {
                0...12 => length,
                13...268 => {
                    header.push((length - 13) as u8);
                    13
                }
                269...64999 => {
                    header.push(((length - 269) >> 8) as u8);
                    header.push((length - 269) as u8);
                    14
                }
                _ => panic!("option too big"),
            } as u8;

            header[0] = base_delta << 4 | base_length;

            *last_option_number = *last_option_number + delta;

            header
        }

        pub fn value_len(&self) -> usize {
            match *self {
                Option::IfMatch(ref v) => (v).len(),
                Option::UriHost(ref s) => s.len(),
                Option::ETag(ref v) => v.len(),
                Option::IfNoneMatch => 0,
                Option::Observe(n) => Self::u32_as_bytes(&n).len(),
                Option::UriPort(n) => Self::u16_as_bytes(&n).len(),
                Option::LocationPath(ref s) => s.len(),
                Option::UriPath(ref s) => s.len(),
                Option::ContentFormat(n) => Self::u16_as_bytes(&n).len(),
                Option::MaxAge(n) => Self::u32_as_bytes(&n).len(),
                Option::UriQuery(ref s) => s.len(),
                Option::Accept(n) => Self::u16_as_bytes(&n).len(),
                Option::LocationQuery(ref s) => s.len(),
                Option::ProxyUri(ref s) => s.len(),
                Option::ProxyScheme(ref s) => s.len(),
                Option::Size1(n) => Self::u32_as_bytes(&n).len(),
                Option::NoResponse(n) => Self::u8_as_bytes(&n).len(),
                Option::Unknown((_, ref v)) => v.len(),
            }
        }

        pub fn value_to_bytes(&self) -> Vec<u8> {
            match *self {
                Option::IfMatch(ref v) => v.to_vec(),
                Option::UriHost(ref s) => s.as_bytes().to_vec(),
                Option::ETag(ref v) => v.to_vec(),
                Option::IfNoneMatch => Vec::with_capacity(0),
                Option::Observe(ref n) => Self::integer_to_bytes(*n as u64),
                Option::UriPort(ref n) => Self::integer_to_bytes(*n as u64),
                Option::LocationPath(ref s) => s.as_bytes().to_vec(),
                Option::UriPath(ref s) => s.as_bytes().to_vec(),
                Option::ContentFormat(ref n) => Self::integer_to_bytes(*n as u64),
                Option::MaxAge(ref n) => Self::integer_to_bytes(*n as u64),
                Option::UriQuery(ref s) => s.as_bytes().to_vec(),
                Option::Accept(ref n) => Self::integer_to_bytes(*n as u64),
                Option::LocationQuery(ref s) => s.as_bytes().to_vec(),
                Option::ProxyUri(ref s) => s.as_bytes().to_vec(),
                Option::ProxyScheme(ref s) => s.as_bytes().to_vec(),
                Option::Size1(ref n) => Self::integer_to_bytes(*n as u64),
                Option::NoResponse(ref n) => Self::integer_to_bytes(*n as u64),
                Option::Unknown((_, ref v)) => v.to_vec(),
            }
        }

        // fn integer_as_bytes<'a, T: Zero + PartialEq + BitAnd + Shr<u8>>(mut n: T) -> Vec<<T as Shr<u8>>::Output> {//&'a [u8] {
        // let length_needed = 0;
        // let bytes = vec![];
        // let m: <T as Shr<u8>>::Output = T::zero();
        // while n != T::zero() {
        // bytes.push((n & 0xFF));
        // n = T::shr(n, 8);
        // }
        //
        // bytes
        // }
        //

        fn integer_to_bytes(mut n: u64) -> Vec<u8> {
            let mut bytes = vec![];
            while n != 0 {
                bytes.push(n as u8);
                n = n >> 8;
            }

            bytes.reverse();
            bytes
        }

        fn u32_as_bytes<'a>(n: &'a u32) -> &'a [u8] {
            use std::mem;

            let mut i = 0;
            let bytes: &[u8; 4] = unsafe { mem::transmute((n as *const u32) as *const u8) };
            while bytes[i] == 0 {
                i += 1
            }

            &bytes[i..]
        }

        fn u16_as_bytes<'a>(n: &'a u16) -> &'a [u8] {
            use std::mem;

            let mut i = 0;
            let bytes: &[u8; 2] = unsafe { mem::transmute((n as *const u16) as *const u8) };
            while bytes[i] == 0 {
                i += 1
            }

            &bytes[i..]
        }

        fn u8_as_bytes<'a>(n: &'a u8) -> &'a [u8] {
            use std::mem;

            let mut i = 0;
            let bytes: &[u8; 1] = unsafe { mem::transmute((n as *const u8) as *const u8) };
            while bytes[i] == 0 {
                i += 1
            }

            &bytes[i..]
        }

        pub fn from_raw(number: u16, value: &[u8]) -> Option {
            let parsed_value = match format::get_by_number(number) {
                format::Format::Empty => Self::should_be_empty(value),
                format::Format::Opaque(min, max) => Self::should_be_opaque(value, min, max),
                format::Format::UInt(min, max) => Self::should_be_uint(value, min, max),
                format::Format::String(min, max) => Self::should_be_string(value, min, max),
            };

            match (number, parsed_value) {
                (1, value::Value::Opaque(v)) => Option::IfMatch(v),
                (3, value::Value::String(v)) => Option::UriHost(v),
                (4, value::Value::Opaque(v)) => Option::ETag(v),
                (5, value::Value::Empty) => Option::IfNoneMatch,
                (6, value::Value::UInt(v)) => Option::Observe(v as u32),
                (7, value::Value::UInt(v)) => Option::UriPort(v as u16),
                (8, value::Value::String(v)) => Option::LocationPath(v),
                (11, value::Value::String(v)) => Option::UriPath(v),
                (12, value::Value::UInt(v)) => Option::ContentFormat(v as u16),
                (14, value::Value::UInt(v)) => Option::MaxAge(v as u32),
                (15, value::Value::String(v)) => Option::UriQuery(v),
                (17, value::Value::UInt(v)) => Option::Accept(v as u16),
                (20, value::Value::String(v)) => Option::LocationQuery(v),
                (35, value::Value::String(v)) => Option::ProxyUri(v),
                (39, value::Value::String(v)) => Option::ProxyScheme(v),
                (60, value::Value::UInt(v)) => Option::Size1(v as u32),
                (284, value::Value::UInt(v)) => Option::NoResponse(v as u8),
                (_, value::Value::Opaque(v)) => Option::Unknown((number, v)),
                _ => panic!("unhandled option number, format combination"),
            }
        }

        pub fn should_be_empty(value: &[u8]) -> value::Value {
            match value.len() {
                0 => value::Value::Empty,
                _ => value::Value::Opaque(value.to_vec()),
            }
        }

        pub fn should_be_string(value: &[u8], min: u16, max: u16) -> value::Value {
            if value.len() < min as usize || value.len() > max as usize {
                return value::Value::Opaque(value.to_vec());
            }

            match String::from_utf8(value.to_vec()) {
                Ok(s) => value::Value::String(s),
                Err(_) => value::Value::Opaque(value.to_vec()),
            }
        }

        pub fn should_be_uint(value: &[u8], min: u16, max: u16) -> value::Value {
            if value.len() >= min as usize && value.len() <= max as usize {
                let mut num: u64 = 0;
                for byte in value {
                    num = (num << 8) | *byte as u64;
                }
                value::Value::UInt(num)
            } else {
                value::Value::Opaque(value.to_vec())
            }
        }


        pub fn should_be_opaque(value: &[u8], _min: u16, _max: u16) -> value::Value {
            return value::Value::Opaque(value.to_vec());
        }

        pub fn number(&self) -> u16 {
            match *self {
                Option::IfMatch(_) => 1,
                Option::UriHost(_) => 3,
                Option::ETag(_) => 4,
                Option::IfNoneMatch => 5,
                Option::Observe(_) => 6,
                Option::UriPort(_) => 7,
                Option::LocationPath(_) => 8,
                Option::UriPath(_) => 11,
                Option::ContentFormat(_) => 12,
                Option::MaxAge(_) => 14,
                Option::UriQuery(_) => 15,
                Option::Accept(_) => 17,
                Option::LocationQuery(_) => 20,
                Option::ProxyUri(_) => 35,
                Option::ProxyScheme(_) => 39,
                Option::Size1(_) => 60,
                Option::NoResponse(_) => 284,
                Option::Unknown((n, _)) => n,
            }
        }

        pub fn is_critical(&self) -> bool {
            self.number() & 0x01 != 0
        }

        pub fn is_elective(&self) -> bool {
            self.number() & 0x01 == 0
        }

        pub fn is_unsafe_to_forward(&self) -> bool {
            self.number() & 0x02 != 0
        }

        pub fn is_safe_to_forward(&self) -> bool {
            self.number() & 0x02 == 0
        }

        pub fn is_no_cache_key(&self) -> bool {
            self.number() & 0x1e == 0x1c
        }

        pub fn is_cache_key(&self) -> bool {
            self.number() & 0x1e != 0x1c
        }
    }

    pub mod value {
        pub enum Value {
            Empty,
            Opaque(Vec<u8>),
            String(String),
            UInt(u64),
        }
    }

    pub mod format {
        pub enum Format {
            Empty,
            Opaque(u16, u16),
            String(u16, u16),
            UInt(u16, u16),
        }

        // pub fn get_format(option: Option) {
        // match option {
        // Option::UriHost(_) => Format::Opaque(0, 8),
        // Option::UriHost(_) => Format::String(1, 255),
        // Option::ETag(_) => Format::Opaque(0, 0),
        // Option::IfNoneMatch(_) => Format::Empty,
        // Option::Observe(_) => Format::UInt(0, 4), //guess
        // Option::UriPort(_) => Format::UInt(0, 2),
        // Option::LocationPath(_) => Format::String(0, 255),
        // Option::UriPath(_) => Format::String(0, 255),
        // Option::ContentFormat(_) => Format::UInt(0, 2),
        // Option::MaxAge(_) => Format::UInt(0, 4),
        // Option::UriQuery(_) => Format::String(0, 255),
        // Option::Accept(_) => Format::UInt(0, 2),
        // Option::LocationQuery(_) => Format::String(0, 255),
        // Option::ProxyUri(_) => Format::String(0, 1034),
        // Option::ProxyScheme(_) => Format::String(0, 255),
        // Option::Size1(_) => Format::UInt(0, 4),
        // Option::NoResponse(_) => Format::UInt(0, 1),
        // Option::Unknown(_) => Format::Opaque(0, 65535)
        // }
        // }
        //

        pub fn get_by_number(number: u16) -> Format {
            match number {
                1 => Format::Opaque(0, 8),
                3 => Format::String(1, 255),
                4 => Format::Opaque(0, 0),
                5 => Format::Empty,
                6 => Format::UInt(0, 4), //guess
                7 => Format::UInt(0, 2),
                8 => Format::String(0, 255),
                11 => Format::String(0, 255),
                12 => Format::UInt(0, 2),
                14 => Format::UInt(0, 4),
                15 => Format::String(0, 255),
                17 => Format::UInt(0, 2),
                20 => Format::String(0, 255),
                35 => Format::String(0, 1034),
                39 => Format::String(0, 255),
                60 => Format::UInt(0, 4),
                284 => Format::UInt(0, 1),
                _ => Format::Opaque(0, 65535),
            }
        }
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
            pkt.extend(option.value_to_bytes());
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
