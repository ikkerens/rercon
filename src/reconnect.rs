use std::{ops::DerefMut, sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::delay_for};

use crate::{
	connection::SingleConnection,
	error::RconError::{self, BusyReconnecting, IO},
	reconnect::Status::{Connected, Disconnected},
};

enum Status {
	Connected(SingleConnection),
	Disconnected(String),
}

/// Drop-in replacement wrapper of [`Connection`](struct.Connection.html) which intercepts all [`IO errors`](enum.Error.html#variant.IO)
/// returned by [`Connection::exec`](struct.Connection.html#method.exec) to start the reconnection thread, and will opt to return [`BusyReconnecting`](enum.Error.html#variant.BusyReconnecting)
/// instead.
///
/// For further docs, refer to [`Connection`](struct.Connection.html), as it shares the same API.
#[derive(Clone)]
pub struct ReconnectingConnection {
	address: String,
	pass: String,
	connect_timeout: Option<Duration>,

	internal: Arc<Mutex<Status>>,
}

impl ReconnectingConnection {
	/// This function behaves identical to [`Connection::open`](struct.Connection.html#method.open).
	pub async fn open<S: Into<String>>(
		address: S, pass: S, connect_timeout: Option<Duration>,
	) -> Result<Self, RconError> {
		let address = address.into();
		let pass = pass.into();
		let internal = Arc::new(Mutex::new(Connected(
			SingleConnection::open(address.clone(), pass.clone(), connect_timeout).await?,
		)));
		Ok(ReconnectingConnection {
			address,
			pass,
			connect_timeout,
			internal,
		})
	}

	/// This function behaves identical to [`Connection::exec`](struct.Connection.html#method.exec) unless `Err([IO](enum.Error.html#variant.IO))` is returned,
	/// in which case it will start reconnecting and return [`BusyReconnecting`](enum.Error.html#variant.BusyReconnecting) until the connection has been re-established.
	pub async fn exec<S: Into<String>>(&mut self, cmd: S) -> Result<String, RconError> {
		// First, we check if we are actively reconnecting, this must be done within a Mutex
		let result = {
			let mut lock = self.internal.lock().await;
			let connection = match lock.deref_mut() {
				Connected(ref mut c) => c,
				Disconnected(msg) => return Err(BusyReconnecting(msg.clone())),
			};

			connection.exec(cmd).await
		};

		// If we are connected, send the request
		match result {
			Err(e) => match &e {
				IO(_) => Err(self.start_reconnect(e).await),
				_ => Err(e),
			},
			Ok(result) => Ok(result),
		}
	}

	async fn start_reconnect(&self, e: RconError) -> RconError {
		// First, we change the status, which automatically disconnects the old connection
		{
			let mut lock = self.internal.lock().await;
			*lock = Disconnected(e.to_string());
		}

		let clone = self.clone();
		tokio::spawn(async move { clone.reconnect_loop().await });

		BusyReconnecting(e.to_string())
	}

	async fn reconnect_loop(&self) {
		loop {
			match SingleConnection::open(self.address.clone(), self.pass.clone(), self.connect_timeout).await {
				Err(_) => {
					delay_for(Duration::from_secs(1)).await;
				}
				Ok(c) => {
					let mut lock = self.internal.lock().await;
					*lock = Connected(c);
					return;
				}
			}
		}
	}
}
