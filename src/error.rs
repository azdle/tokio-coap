use message::Error as MessageError;
use std::io::Error as IoError;

/// All errors returned from this crate.
#[derive(Debug)]
pub enum Error {
    /// A timeout was reached while waiting for a reply or event
    Timeout,
    /// A message was unable to be parsed successfully.
    Message(MessageError),
    /// The system IO returned an error.
    Io(IoError),

    #[doc(hidden)]
    __AlwaysWildcardMatchThisListWillChange,
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
