use std::fmt;

use etebase::error::Error;

/// An error returned by Etebase. Kotlin: `EtebaseException`, one subclass per variant.
///
/// Flat error: Kotlin receives the variant and its message, which is exactly the Rust message.
#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum EtebaseError {
    /// A generic error
    Generic(String),
    /// An error parsing a URL (e.g. the server URL)
    UrlParse(String),
    /// A msgpack serialization or deserialization error
    MsgPack(String),
    /// The API was used incorrectly
    ProgrammingError(String),
    /// The item's content has not been downloaded yet
    MissingContent(String),
    /// Invalid padding of encrypted content
    Padding(String),
    /// Invalid Base64 input
    Base64(String),
    /// An encryption or decryption failure
    Encryption(String),
    /// The server rejected the credentials
    Unauthorized(String),
    /// A conflict, e.g. a failed transaction
    Conflict(String),
    /// The operation is not permitted
    PermissionDenied(String),
    /// The requested resource does not exist
    NotFound(String),
    /// The request did not complete (DNS, TLS, I/O, or a failing `HttpClient`)
    Connection(String),
    /// A temporary server error (e.g. maintenance or gateway problems)
    TemporaryServerError(String),
    /// The server failed to process the request
    ServerError(String),
    /// Any other HTTP error status
    Http(String),
}

impl fmt::Display for EtebaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Generic(s)
            | Self::UrlParse(s)
            | Self::MsgPack(s)
            | Self::ProgrammingError(s)
            | Self::MissingContent(s)
            | Self::Padding(s)
            | Self::Base64(s)
            | Self::Encryption(s)
            | Self::Unauthorized(s)
            | Self::Conflict(s)
            | Self::PermissionDenied(s)
            | Self::NotFound(s)
            | Self::Connection(s)
            | Self::TemporaryServerError(s)
            | Self::ServerError(s)
            | Self::Http(s) => s,
        };
        f.write_str(message)
    }
}

impl std::error::Error for EtebaseError {}

impl From<Error> for EtebaseError {
    fn from(err: Error) -> Self {
        match err {
            Error::Generic(s) => Self::Generic(s),
            Error::UrlParse(s) => Self::UrlParse(s),
            Error::MsgPack(s) => Self::MsgPack(s),
            Error::ProgrammingError(s) => Self::ProgrammingError(s.to_owned()),
            Error::MissingContent(s) => Self::MissingContent(s.to_owned()),
            Error::Padding(s) => Self::Padding(s.to_owned()),
            Error::Base64(s) => Self::Base64(s.to_owned()),
            Error::Encryption(s) => Self::Encryption(s.to_owned()),
            Error::Unauthorized(s) => Self::Unauthorized(s),
            Error::Conflict(s) => Self::Conflict(s),
            Error::PermissionDenied(s) => Self::PermissionDenied(s),
            Error::NotFound(s) => Self::NotFound(s),
            Error::Connection(s) => Self::Connection(s),
            Error::TemporaryServerError(s) => Self::TemporaryServerError(s),
            Error::ServerError(s) => Self::ServerError(s),
            Error::Http(s) => Self::Http(s),
        }
    }
}

pub type Result<T, E = EtebaseError> = std::result::Result<T, E>;
