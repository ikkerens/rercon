//! ReRCON is a RCON library written with primarily the game Ark: Survival Evolved in mind.
//! While this library is Valve RCON compliant, and should thus work with other RCON applications, it has not been tested for other applications.
//!
//! There are two primary types to be used with this create,
//! [`Connection`](struct.Connection.html) and [`ReConnection`](struct.ReConnection.html),
//! both of these types share the same API,
//! the primary difference is that [`ReConnection::exec`](struct.ReConnection.html#method.exec) will never return [`IO errors`](enum.Error.html#variant.IO),
//! as it will start a new thread to reconnect,
//! instead, it will return error [`BusyReconnecting`](enum.Error.html#variant.BusyReconnecting),
//! with a string being a `to_string` representation of the error that caused the reconnect in the first place.
//!
//! All public methods use a template to accept all forms of strings that implement `Into<String>`, however the library will always return `std::string::String`

#![deny(warnings, bad_style, missing_docs)]

pub use crate::connection::Settings;
pub use crate::connection::SingleConnection as Connection;
pub use crate::error::RconError as Error;
#[cfg(feature = "reconnection")]
pub use crate::reconnect::ReconnectingConnection as ReConnection;

mod connection;
mod error;
mod packet;
mod packet_net;
#[cfg(feature = "reconnection")]
mod reconnect;

#[cfg(test)]
mod tests;
