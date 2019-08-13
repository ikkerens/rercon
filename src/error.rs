use std::{io, net};
use std::fmt::{Debug, Display, Formatter};
use std::string::FromUtf8Error;

use crate::error::RconError::{AddressParseError, IOError, UTFEncodingError};

#[derive(Debug)]
pub enum RconError {
    AddressParseError(net::AddrParseError),
    IOError(io::Error),
    CommandTooLongError,
    UTFEncodingError(FromUtf8Error),
    UnexpectedPacketError,
    DesynchronizedPacketError,
    PasswordIncorrectError,
    BusyReconnecting(String),
}

impl ::std::error::Error for RconError {
    fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
        match self {
            IOError(e) => Some(e),
            AddressParseError(e) => Some(e),
            UTFEncodingError(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for RconError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        (self as &dyn Debug).fmt(f)
    }
}

impl From<io::Error> for RconError {
    fn from(e: io::Error) -> Self {
        IOError(e)
    }
}

impl From<net::AddrParseError> for RconError {
    fn from(e: net::AddrParseError) -> Self {
        AddressParseError(e)
    }
}

impl From<FromUtf8Error> for RconError {
    fn from(e: FromUtf8Error) -> Self {
        UTFEncodingError(e)
    }
}