use message::Error as MessageError;
use std::io::Error as IoError;
use std::error::Error as StdError;

/// All errors returned from this crate.
#[derive(Debug)]
pub enum Error {
    /// A timeout was reached while waiting for a reply or event
    Timeout,
    /// A message was unable to be parsed successfully.
    Message(MessageError),
    /// The system IO returned an error.
    Io(IoError),
    /// Generic error when attempting to parse a url
    // TODO: Some of the specific errors should be named, maybe a separate enum
    // for the errors encountered during url parsing should be used
    UrlParsing(Box<StdError + Send + Sync>),

    #[doc(hidden)]
    __AlwaysWildcardMatchThisListWillChange,
}

impl Error {
    pub(crate) fn url_parsing(err: impl Into<Box<StdError + Send + Sync>>) -> Error {
        Error::UrlParsing(err.into())
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
