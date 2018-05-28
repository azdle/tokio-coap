use std::collections::BTreeMap;
use std::borrow::Cow;
use std::str;
use message::Error;

use std::option::Option as StdOption;

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub map: BTreeMap<u16, Vec<Vec<u8>>>,
}

impl Options {
    pub fn new() -> Self {
        Options {
            map: BTreeMap::new(),
        }
    }

    pub fn iter(&self) -> RawOptionsIterator {
        RawOptionsIterator::new(self)
    }

    pub fn push<T: Option + Byteable>(&mut self, option: T) {
        self.map
            .entry(option.number())
            .or_insert_with(|| Vec::new())
            .push(option.to_bytes().into_owned());
    }

    pub fn push_raw(&mut self, number: u16, raw_value: Vec<u8>) {
        self.map
            .entry(number)
            .or_insert_with(|| Vec::new())
            .push(raw_value);
    }

    pub fn get<T: Option>(&self) -> StdOption<Vec<T>> {
        self.map
            .get(&<T as Option>::NUMBER)
            .map(|o| o.iter()
                      .map(|v| <T as Option>::from_bytes(v.as_ref()).unwrap() )
                      .collect())
    }

    pub fn get_raw<T: Option>(&self) -> StdOption<Vec<Vec<u8>>> {
        self.map
            .get(&<T as Option>::NUMBER)
            .map(|v| v.to_owned() )
    }
}

pub struct RawOptionsIterator<'a> {
    options: &'a Options,
    place: usize
}

impl<'a> RawOptionsIterator<'a> {
    fn new(options: &'a Options) -> RawOptionsIterator<'a> {
        RawOptionsIterator {
            options: options,
            place: 0,
        }
    }
}

impl<'a> Iterator for RawOptionsIterator<'a> {
    type Item = (u16, &'a [u8]);

    fn next(&mut self) -> StdOption<Self::Item> {
        let i = self.place;
        self.place += 1;
        self.options.map.iter().flat_map(|(&k, ref v)| v.iter().map(move |v| (k,v.as_ref()))).nth(i)
    }
}

pub trait Option: Sized {
    const NUMBER: u16;
    type Format;

    fn new(Self::Format) -> Self;
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

pub trait Byteable {
    fn number(&self) -> u16;

    fn to_bytes(&self) -> Cow<[u8]>;
    fn bytes_len(&self) -> usize;
    // TODO: add as_bytes, into_bytes
}

pub fn build_header<'a>(number: u16, bytes: &[u8], last_option_number: &mut u16) -> Cow<'a, [u8]> {
    let mut header = vec![0u8];

    if number < *last_option_number {
        panic!("bad order");
    }

    let delta = number - *last_option_number;
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
    let length = bytes.len();
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

    Cow::Owned(header)
}

/// This builds the full type for each individual option.
macro_rules! option {
    // Opaque Type Options
    ($num: expr, $name: ident, opaque, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: Vec<u8>
        }

        impl Option for $name {
            const NUMBER: u16 = $num;
            type Format = Vec<u8>;

            fn new(value: Self::Format) -> Self {
                $name{value: value}
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    Ok(Self{value: bytes.to_vec()})
                } else {
                    Err(Error::MessageFormat)
                }
            }
        }

        impl Byteable for $name {
            fn number(&self) -> u16 {
                $num
            }

            fn to_bytes(&self) -> Cow<[u8]> {
                Cow::Owned(self.value.clone())
            }

            fn bytes_len(&self) -> usize {
                self.value.len()
            }

        }

        impl<'a> From<&'a [u8]> for $name {
            fn from(bytes: &'a [u8]) -> Self {
                Self {
                    value: bytes.to_vec()
                }
            }
        }
    };

    // String Type Options
    ($num: expr, $name: ident, string, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            pub value: String
        }

        impl Option for $name {
            const NUMBER: u16 = $num;
            type Format = String;

            fn new(value: Self::Format) -> Self {
                $name{value: value}
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    Ok($name{value: str::from_utf8(bytes).or(Err(Error::MessageFormat))?.to_string()})
                } else {
                    Err(Error::MessageFormat)
                }
            }

        }

        impl Byteable for $name {
            fn number(&self) -> u16 {
                $num
            }

            fn to_bytes(&self) -> Cow<[u8]> {
                Cow::Owned(self.value.clone().into_bytes())
            }

            fn bytes_len(&self) -> usize {
                self.value.bytes().len()
            }
        }

        impl<'a> From<&'a str> for $name {
            fn from(str: &'a str) -> Self {
                Self {
                    value: str.to_owned()
                }
            }
        }
    };

    // Empty Type Options
    ($num: expr, $name: ident, empty, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name;

        impl Option for $name {
            const NUMBER: u16 = $num;
            type Format = ();

            fn new(_value: ()) -> Self {
                $name
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() != 0 {
                    Ok($name)
                } else {
                    Err(Error::MessageFormat)
                }
            }

        }

        impl Byteable for $name {
            fn number(&self) -> u16 {
                $num
            }

            fn to_bytes(&self) -> Cow<[u8]> {
                Cow::Borrowed(&[])
            }

            fn bytes_len(&self) -> usize {
                0
            }
        }

        //TODO: Impl From for (), somehow
    };

    // UInt Type Options
    ($num: expr, $name: ident, uint, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: u64
        }

        impl Option for $name {
            const NUMBER: u16 = $num;
            type Format = u64;

            fn new(value: u64) -> Self {
                $name{value: value}
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() >= $min as usize && bytes.len() <= $max as usize {
                    Ok($name{value: bytes_to_value(bytes)})
                } else {
                    Err(Error::MessageFormat)
                }
            }

        }

        impl Byteable for $name {
            fn number(&self) -> u16 {
                $num
            }

            fn to_bytes(&self) -> Cow<[u8]> {

                Cow::Owned(value_to_bytes(self.value))
            }

            fn bytes_len(&self) -> usize {
                let mut n = self.value;
                let mut i = 0;

                while n != 0 {
                    i+=1;
                    n = n >> 8;
                }

                i
            }
        }

        impl<'a> From<&'a [u8]> for $name {
            fn from(bytes: &'a [u8]) -> Self {
                Self {
                    value: bytes_to_value(bytes)
                }
            }
        }
    }
}

// Helpers

// TODO: Replace with something like byte order?
fn bytes_to_value(bytes: &[u8]) -> u64 {
    let mut value = 0u64;

    for byte in bytes {
        value = (value << 8) + *byte as u64;
    }

    value
}

fn value_to_bytes(mut n: u64) -> Vec<u8> {
    let mut bytes = vec![];
    while n != 0 {
        bytes.push(n as u8);
        n = n >> 8;
    }

    bytes.reverse();
    bytes
}


/// This builds the type for each individual option.
macro_rules! options {
    ( $( ($num: expr, $name: ident, $format: ident, $min: expr, $max: expr), )+ ) => {
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

