use std::collections::BTreeMap;
use std::borrow::Cow;
use std::str;
use message::Error;

use std::option::Option as StdOption;

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub map: BTreeMap<OptionKind, Vec<OptionType>>,
}

impl Options {
    pub fn new() -> Self {
        Options {
            map: BTreeMap::new(),
        }
    }

    pub fn iter(&self) -> OptionsIterator {
        OptionsIterator::new(self)
    }

    pub fn push(&mut self, option: OptionType) {
        self.map
            .entry(option.kind())
            .or_insert_with(|| Vec::new())
            .push(option);
    }

    pub fn get_all_of(&mut self, kind: OptionKind) -> StdOption<&Vec<OptionType>> {
        self.map
            .get(&kind)
    }

//    pub fn get<T: Option>(&mut self) -> StdOption<&Vec<T>> {
//        let kind = <T as Option>::kind(T);
//        self.map
//            .get(&kind)
//    }
}

pub struct OptionsIterator<'a> {
    options: &'a Options,
    place: usize
}

impl<'a> OptionsIterator<'a> {
    fn new(options: &'a Options) -> OptionsIterator<'a> {
        OptionsIterator {
            options: options,
            place: 0,
        }
    }
}

impl<'a> Iterator for OptionsIterator<'a> {
    type Item = &'a Byteable;

    fn next(&mut self) -> StdOption<Self::Item> {
        let i = self.place;
        self.place += 1;
        self.options.map.iter().flat_map(|(_k,v)| v).nth(i).map(|ot| ot.as_byteable())
    }
}

impl IntoIterator for Options {
    type Item = OptionType;
    type IntoIter = Box<Iterator<Item=OptionType>>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.map.into_iter().flat_map(|(_k,v)| v))
    }
}

pub trait Option: Sized {
    type Value;

    fn kind(&self) -> OptionKind;

    fn new(Self::Value) -> Self;

    fn into_type(self) -> OptionType;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

pub trait Byteable {
    fn number(&self) -> u16;

    fn to_bytes(&self) -> Cow<[u8]>;
    fn bytes_len(&self) -> usize;
    // TODO: add as_bytes, into_bytes
}

pub fn build_header<'a>(option: &'a Byteable, last_option_number: &mut u16) -> Cow<'a, [u8]> {
    let mut header = vec![0u8];

    if option.number() < *last_option_number {
        panic!("bad order");
    }

    let delta = option.number() - *last_option_number;
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
    let length = option.bytes_len();
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

/// This macro contains the common structure of individual option types.
macro_rules! option_common_fns {
    ($name: ident) => {
        fn kind(&self) -> OptionKind {
            OptionKind::$name
        }

        fn into_type(self) -> OptionType {
            OptionType::$name(self)
        }
    };

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
            type Value = Vec<u8>;
            option_common_fns!($name);

            fn new(value: Self::Value) -> Self {
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
    };

    // String Type Options
    ($num: expr, $name: ident, string, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: String
        }

        impl Option for $name {
            type Value = String;
            option_common_fns!($name);

            fn new(value: Self::Value) -> Self {
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
    };

    // Empty Type Options
    ($num: expr, $name: ident, empty, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name;

        impl Option for $name {
            type Value = ();
            option_common_fns!($name);

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
    };

    // UInt Type Options
    ($num: expr, $name: ident, uint, $min: expr, $max: expr) => {
        #[derive(PartialEq, Eq, Debug)]
        pub struct $name {
            value: u64
        }

        impl Option for $name {
            type Value = u64;
            option_common_fns!($name);

            fn new(value: u64) -> Self {
                $name{value: value}
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                // TODO: Replace with something like byte order?
                fn bytes_to_value(bytes: &[u8]) -> u64 {
                    let mut value = 0u64;

                    for byte in bytes {
                        value = (value << 8) + *byte as u64;
                    }

                    value
                }

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
                fn value_to_bytes(mut n: u64) -> Vec<u8> {
                    let mut bytes = vec![];
                    while n != 0 {
                        bytes.push(n as u8);
                        n = n >> 8;
                    }

                    bytes.reverse();
                    bytes
                }

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

        #[derive(PartialEq, Eq, Debug)]
        pub enum OptionType {
            $(
                $name($name),
            )+
            Unknown(Unknown)
        }

        impl OptionType {
            fn kind(&self) -> OptionKind {
                match *self {
                    $(
                        OptionType::$name(_) => OptionKind::$name,
                    )+
                    OptionType::Unknown(ref o) => OptionKind::Unknown(o.number())
                }
            }

            pub fn number(&self) -> u16 {
                match *self {
                    $(
                        OptionType::$name(_) => $num,
                    )+
                    OptionType::Unknown(ref o) => o.number()
                }
            }

            pub fn as_byteable(&self) -> &Byteable {
                match *self {
                    $(
                        OptionType::$name(ref o) => { o as &Byteable },
                    )+
                    OptionType::Unknown(ref o) => { o as &Byteable },
                }
            }
        }

        $(
            impl From<$name> for OptionType {
                fn from(option: $name) -> OptionType {
                    OptionType::$name(option)
                }
            }
        )+


        pub fn from_raw(number: u16, v: &[u8]) -> Result<OptionType, Error> {
            Ok(match number {
                $(
                    $num => { let o = $name::from_bytes(v)?; OptionType::$name(o) },
                )+
                _ => { let mut o = Unknown::from_bytes(v)?; o.set_number(number); OptionType::Unknown(o) },
            })
        }

        $(
            option!($num, $name, $format, $min, $max);
        )+

        //;


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

#[derive(PartialEq, Eq, Debug)]
pub struct Unknown {
    number: u16,
    value: Vec<u8>
}

impl Unknown {
    fn set_number(&mut self, number: u16) {
        self.number = number;
    }
}

impl Option for Unknown {
    type Value = Vec<u8>;

    fn kind(&self) -> OptionKind {
        OptionKind::Unknown(self.number)
    }

    fn into_type(self) -> OptionType {
        OptionType::Unknown(self)
    }

    fn new(value: Self::Value) -> Self {
        Unknown{value: value, number: 0}
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self{value: bytes.to_vec(), number: 0})
    }
}

impl Byteable for Unknown {
    fn number(&self) -> u16 {
        self.number
    }

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(self.value.clone())
    }

    fn bytes_len(&self) -> usize {
        self.value.len()
    }

}

