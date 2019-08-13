pub use crate::connection::SingleConnection as Connection;
pub use crate::reconnect::ReconnectingConnection as ReConnection;

mod error;
mod packet;
mod connection;
mod reconnect;

#[cfg(test)]
mod tests;
