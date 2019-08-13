use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::connection::SingleConnection;
use crate::error::RconError;
use crate::error::RconError::{BusyReconnecting, IOError};
use crate::reconnect::Status::{Connected, Disconnected};

enum Status {
    Connected(SingleConnection),
    Disconnected(String),
}

#[derive(Clone)]
pub struct ReconnectingConnection {
    address: String,
    pass: String,
    connect_timeout: Option<Duration>,

    internal: Arc<Mutex<Status>>,
}

impl ReconnectingConnection {
    pub fn open<S: Into<String>>(address: S, pass: S, connect_timeout: Option<Duration>) -> Result<Self, RconError> {
        let address = address.into();
        let pass = pass.into();
        let internal = Arc::new(Mutex::new(Connected(SingleConnection::open(address.clone(), pass.clone(), connect_timeout)?)));
        Ok(ReconnectingConnection {
            address,
            pass,
            connect_timeout,
            internal,
        })
    }

    pub fn exec<S: Into<String>>(&mut self, cmd: S) -> Result<String, RconError> {
        // First, we check if we are actively reconnecting, this must be done within a Mutex
        let result = {
            let mut lock = self.internal.lock().unwrap();
            let connection = match lock.deref_mut() {
                Connected(ref mut c) => c,
                Disconnected(msg) => return Err(BusyReconnecting(msg.clone())),
            };

            connection.exec(cmd)
        };

        // If we are connected, send the request
        match result {
            Err(e) => match &e {
                IOError(_) => Err(self.start_reconnect(e)),
                _ => Err(e)
            },
            Ok(result) => Ok(result)
        }
    }

    fn start_reconnect(&self, e: RconError) -> RconError {
        // First, we change the status, which automatically disconnects the old connection
        {
            let mut lock = self.internal.lock().unwrap();
            *lock = Disconnected(e.to_string());
        }

        let clone = self.clone();
        thread::spawn(move || clone.reconnect_loop());

        BusyReconnecting(e.to_string())
    }

    fn reconnect_loop(&self) {
        loop {
            match SingleConnection::open(self.address.clone(), self.pass.clone(), self.connect_timeout.clone()) {
                Err(_) => {
                    thread::sleep(Duration::from_secs(1));
                }
                Ok(c) => {
                    let mut lock = self.internal.lock().unwrap();
                    *lock = Connected(c);
                    return;
                }
            }
        }
    }
}
