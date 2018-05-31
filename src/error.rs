use message::Error as MessageError;
use std::io::Error as IoError;
use std::str::Utf8Error;
use uri::ParseError;

#[derive(Debug)]
pub enum UrlError {
    /// The supplied string could not be parsed as a Uri
    Parse(ParseError),
    /// The path was not a valid utf8 string after percent-decoding
    NonUtf8(Utf8Error),
    /// The scheme was not coap
    UnsupportedScheme(String),
    /// The Uri specified a non-absolute path
    NonAbsolutePath,
    /// The Uri included a fragment
    FragmentSpecified,

    #[doc(hidden)]
    __AlwaysWildcardMatchThisListMayChange,
}

/// All errors returned from this crate.
#[derive(Debug)]
pub enum Error {
    /// A timeout was reached while waiting for a reply or event
    Timeout,
    /// A message was unable to be parsed successfully.
    Message(MessageError),
    /// The system IO returned an error.
    Io(IoError),
    /// Error when attempting to parse a url
    Url(UrlError),

    #[doc(hidden)]
    __AlwaysWildcardMatchThisListWillChange,
}

impl From<UrlError> for Error {
    fn from(e: UrlError) -> Error {
        Error::Url(e)
    }
}

impl From<MessageError> for Error {
    fn from(e: MessageError) -> Error {
        Error::Message(e)
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Error {
        Error::Io(e)
    }
}
