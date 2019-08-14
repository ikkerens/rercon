use std::{net::{SocketAddr, TcpStream},
          time::Duration};

use crate::{error::{RconError,
                    RconError::{DesynchronizedPacket, PasswordIncorrect, UnexpectedPacket}},
            packet::{Packet, TYPE_AUTH, TYPE_AUTH_RESPONSE, TYPE_EXEC, TYPE_RESPONSE}};

/// Represents a single-established RCON connection to the server, which will not automatically reconnect once the connection has failed.
/// This struct will instead opt to return [`IO errors`](enum.Error.html#variant.IO), leaving connection responsibility in the callers hands.
///
/// # Example
/// ```rust,no_run
/// use rercon::Connection;
///
/// let mut connection = Connection::open("123.456.789.123:27020", "my_secret_password", None).unwrap();
/// let reply = connection.exec("hello").unwrap();
/// println!("Reply from server: {}", reply);
/// ```
pub struct SingleConnection {
    stream: TcpStream,
    counter: i32,
}

impl SingleConnection {
    /// Opens a new RCON connection, with an optional timeout, and authenticates the connection to the remote server.
    pub fn open<S: Into<String>>(address: S, pass: S, connect_timeout: Option<Duration>) -> Result<Self, RconError> {
        let mut stream = {
            let socket_address: SocketAddr = address.into().parse()?;
            match connect_timeout {
                Some(timeout) => TcpStream::connect_timeout(&socket_address, timeout),
                None => TcpStream::connect(&socket_address),
            }?
        };

        Packet::new(0, TYPE_AUTH, pass.into()).send_internal(&mut stream)?;
        {
            let response = Packet::read(&mut stream)?;
            if *response.get_packet_type() != TYPE_AUTH_RESPONSE {
                return Err(UnexpectedPacket);
            }
            if *response.get_id() == -1 {
                return Err(PasswordIncorrect);
            }
        }

        Ok(Self {
            stream,
            counter: 0,
        })
    }

    /// Sends a command to the RCON server, returning the combined reply (in case there are multiple packets) or an error.
    pub fn exec<S: Into<String>>(&mut self, cmd: S) -> Result<String, RconError> {
        // 0. Prepare some variables
        let mut end_id = -1;
        let mut result = String::new();

        // 1. Send the original command
        let original_id = self.next_counter();
        Packet::new(original_id, TYPE_EXEC, cmd.into()).send_internal(&mut self.stream)?;

        // 2. Loop until we have confirmation the message is complete
        loop {
            // 3. Read the first response to the command
            let response = Packet::read(&mut self.stream)?;

            // 4. After the first read, we send an empty command, which should be mirrored.
            //    We do this because some RCON servers don't properly respond if we send execs
            //    too fast. So we wait for the first response.
            if end_id == -1 { // Our counter can never be negative due to overflow protection
                end_id = self.next_counter();
                Packet::new(end_id, TYPE_EXEC, "").send_internal(&mut self.stream)?;
            }

            // 5. Check if we received the correct ID, if not, something thread-fucky went on.
            if *response.get_id() != original_id && *response.get_id() != end_id {
                return Err(DesynchronizedPacket);
            }

            // 6. We should only be receiving a response at this time.
            if *response.get_packet_type() != TYPE_RESPONSE {
                return Err(UnexpectedPacket);
            }

            // 7. If we receive a response to our empty command from 4, that means all previous
            //    messages have been sent and (hopefully) received. That means we can finish up our
            //    result.
            if *response.get_id() == end_id {
                break;
            }

            // 8. All checks have passed, append body to the end result
            result += response.get_body().as_str();
        }

        // Thank you come again
        Ok(result)
    }

    fn next_counter(&mut self) -> i32 {
        self.counter = match self.counter.checked_add(1) {
            Some(new) => new,
            None => 1,
        };
        self.counter
    }
}
