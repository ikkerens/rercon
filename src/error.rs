use std::{
	fmt::{Debug, Display, Formatter},
	string::FromUtf8Error,
	{io, net},
};

use crate::error::RconError::{AddressParse, UTFEncoding, IO};

/// A common error enum that is returned by all public functions describing different forms of failures that can occur within this library.
#[derive(Debug)]
pub enum RconError {
	/// There is an error in the passed address field
	AddressParse(net::AddrParseError),
	/// There was a network issue during connection or exec
	IO(io::Error),
	/// The command provided is longer than 1014 characters.
	CommandTooLong,
	/// The server did not respond with proper UTF-8
	UTFEncoding(FromUtf8Error),
	/// The server sent a packet with a type we were not expecting.
	UnexpectedPacket,
	/// The pass field is incorrect
	PasswordIncorrect,
	/// Returned by [`ReConnection::exec`](struct.ReConnection.html#method.exec) when [`ReConnection`](struct.ReConnection.html) is busy reconnecting.
	BusyReconnecting(String),
}

impl ::std::error::Error for RconError {
	fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
		match self {
			IO(e) => Some(e),
			AddressParse(e) => Some(e),
			UTFEncoding(e) => Some(e),
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
		IO(e)
	}
}

impl From<net::AddrParseError> for RconError {
	fn from(e: net::AddrParseError) -> Self {
		AddressParse(e)
	}
}

impl From<FromUtf8Error> for RconError {
	fn from(e: FromUtf8Error) -> Self {
		UTFEncoding(e)
	}
}
