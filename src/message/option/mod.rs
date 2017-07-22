use std::collections::BTreeMap;
use std::borrow::Cow;
use std::str;
use message::Error;

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

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    map: BTreeMap<OptionKind, OptionType>
}

impl Options {
    pub fn new() -> Self {
        Options {
            map: BTreeMap::new()
        }
    }

    pub fn push<T: OptionTr>(&mut self, option: T) {

    }
}

trait OptionTr {
    /// NOTE: This should be replaced with an associated const when they make it to stable.
    fn number() -> u16;

    fn new() -> Self;

    fn push_value_from_bytes(&mut self, bytes: &[u8]) -> Result<&mut Self, Error>;
    fn pop_value_to_bytes(&mut self) -> ::std::option::Option<Cow<[u8]>>;
}

// fn get_number<T: OptionTr>(option: T) -> u16 {
//     OptionTr::number(&option)
// }

/// This builds the type for each individual option.
macro_rules! option {
    ($num: expr, $name: ident, opaque, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: Vec<Vec<u8>>
        }

        impl OptionTr for $name {
            fn number() -> u16 {
                $num
            }

            fn new() -> Self {
                $name{value: Vec::new()}
            }

            fn push_value_from_bytes(&mut self, bytes: &[u8]) -> Result<&mut Self, Error> {
                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    self.value.push(bytes.to_vec());
                    Ok(self)
                } else {
                    Err(Error::MessageFormat)
                }
            }

            fn pop_value_to_bytes(&mut self) -> ::std::option::Option<Cow<[u8]>> {
                self.value.pop().map(|value| Cow::Owned(value))
            }
        }
    };

    ($num: expr, $name: ident, string, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: Vec<String>
        }

        impl OptionTr for $name {
            fn number() -> u16 {
                $num
            }

            fn new() -> Self {
                $name{value: Vec::new()}
            }

            fn push_value_from_bytes(&mut self, bytes: &[u8]) -> Result<&mut Self, Error> {
                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    self.value.push(str::from_utf8(bytes).or(Err(Error::MessageFormat))?.to_string());
                    Ok(self)
                } else {
                    Err(Error::MessageFormat)
                }
            }

            fn pop_value_to_bytes(&mut self) -> ::std::option::Option<Cow<[u8]>> {
                self.value.pop().map(|value| Cow::Owned(value.into_bytes()))
            }
        }
    };

    ($num: expr, $name: ident, empty, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: Vec<[(); 0]> // TODO: This should probably just be a counter, maybe?
        }

        impl OptionTr for $name {
            fn number() -> u16 {
                $num
            }

            fn new() -> Self {
                $name{value: Vec::new()}
            }

            fn push_value_from_bytes(&mut self, bytes: &[u8]) -> Result<&mut Self, Error> {
                if bytes.len() != 0 {
                    self.value.push([]);
                    Ok(self)
                } else {
                    Err(Error::MessageFormat)
                }
            }

            fn pop_value_to_bytes(&mut self) -> ::std::option::Option<Cow<[u8]>> {
                self.value.pop().map(|_| Cow::Owned(Vec::new()))
            }
        }
    };

    ($num: expr, $name: ident, uint, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: Vec<u64>
        }

        impl OptionTr for $name {
            fn number() -> u16 {
                $num
            }

            fn new() -> Self {
                $name{value: Vec::new()}
            }

            fn push_value_from_bytes(&mut self, bytes: &[u8]) -> Result<&mut Self, Error> {
                // TODO: Replace with something like byte order?
                fn bytes_to_value(bytes: &[u8]) -> u64 {
                    let mut value = 0u64;

                    for byte in bytes {
                        value = (value << 8) + *byte as u64;
                    }

                    value
                }

                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    self.value.push(bytes_to_value(bytes));
                    Ok(self)
                } else {
                    Err(Error::MessageFormat)
                }
            }

            fn pop_value_to_bytes(&mut self) -> ::std::option::Option<Cow<[u8]>> {
                fn value_to_bytes(mut n: u64) -> Vec<u8> {
                    let mut bytes = vec![];
                    while n != 0 {
                        bytes.push(n as u8);
                        n = n >> 8;
                    }

                    bytes.reverse();
                    bytes
                }

                self.value.pop().map(|value| Cow::Owned(value_to_bytes(value)))
            }
        }
    }
}

/// This builds the type for each individual option.
macro_rules! options {
    ( $( ($num: expr, $name: ident, $format: ident, $min: expr, $max: expr), )+ ) => {
         #[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
        pub enum OptionKind {
            $(
                $name,
            )+
            Unknown(u16)
        }

        impl OptionKind {
            fn from_number(number: u16) -> Self {
                match number {
                    $(
                        $num => OptionKind::$name,
                    )+
                    _ => OptionKind::Unknown(number),
                }
            }

            fn to_number(kind: Self) -> u16 {
                match kind {
                    $(
                        OptionKind::$name => $num,
                    )+
                    OptionKind::Unknown(number) => number,
                }
            }
        }

        #[derive(PartialEq, Eq, Debug)]
        pub enum OptionType {
            $(
                $name($name),
            )+
            Unknown(u16, Vec<u8>)
        }

        pub fn from_raw(number: u16, v: &[u8]) -> Result<OptionType, Error> {
            Ok(match number {
                $(
                    $num => { let mut o = $name::new(); o.push_value_from_bytes(v).unwrap(); OptionType::$name(o) },
                )+
                _ => OptionType::Unknown(number, v.to_vec()),
            })
        }

        $(
            option!($num, $name, $format, $min, $max);
        )+
    }
}

options![
    (1, IfMatch, opaque, 0, 8),
    (3, UriHost, string, 1, 8),
    (4, ETag, opaque, 0, 8),
    (5, IfNoneMatch, empty, -1, -1), // TODO: fix macro to not need this
    (6, Observe, uint, 0, 4),
    (7, UriPort, uint, 0, 2),
    (8, LocationPath, string, 0, 255),
    (11, UriPath, string, 0, 255),
    (12, ContentFormat, uint, 0, 2),
    (14, MaxAge, uint, 0, 4),
    (15, UriQuery, string, 0, 255),
    (17, Accept, uint, 0, 2),
    (20, LocationQuery, string, 0, 255),
    (35, ProxyUri, string, 1, 1034),
    (29, ProxyScheme, string, 1, 255),
    (60, Size1, uint, 0, 4),
    (284, NoResponse, uint, 0, 1),
];

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
            Option::Observe(n) => Self::integer_to_bytes(n as u64).len(),
            Option::UriPort(n) => Self::integer_to_bytes(n as u64).len(),
            Option::LocationPath(ref s) => s.len(),
            Option::UriPath(ref s) => s.len(),
            Option::ContentFormat(n) => Self::integer_to_bytes(n as u64).len(),
            Option::MaxAge(n) => Self::integer_to_bytes(n as u64).len(),
            Option::UriQuery(ref s) => s.len(),
            Option::Accept(n) => Self::integer_to_bytes(n as u64).len(),
            Option::LocationQuery(ref s) => s.len(),
            Option::ProxyUri(ref s) => s.len(),
            Option::ProxyScheme(ref s) => s.len(),
            Option::Size1(n) => Self::integer_to_bytes(n as u64).len(),
            Option::NoResponse(n) => Self::integer_to_bytes(n as u64).len(),
            Option::Unknown((_, ref v)) => v.len(),
        }
    }

    pub fn value_to_bytes(&self) -> Cow<[u8]> {
        match *self {
            Option::IfMatch(ref v) => Cow::Borrowed(v),
            Option::UriHost(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::ETag(ref v) => Cow::Borrowed(v),
            Option::IfNoneMatch => Cow::Owned(Vec::with_capacity(0)),
            Option::Observe(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::UriPort(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::LocationPath(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::UriPath(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::ContentFormat(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::MaxAge(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::UriQuery(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::Accept(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::LocationQuery(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::ProxyUri(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::ProxyScheme(ref s) => Cow::Borrowed(s.as_bytes()),
            Option::Size1(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::NoResponse(ref n) => Cow::Owned(Self::integer_to_bytes(*n as u64)),
            Option::Unknown((_, ref v)) => Cow::Borrowed(v),
        }
    }

    fn integer_to_bytes(mut n: u64) -> Vec<u8> {
        let mut bytes = vec![];
        while n != 0 {
            bytes.push(n as u8);
            n = n >> 8;
        }

        bytes.reverse();
        bytes
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

