use std::{mem, ops::DerefMut, panic::panic_any, sync::Arc, time::Duration};

use tokio::{
	select,
	sync::{Mutex, Notify},
	task::JoinHandle,
	time::delay_for,
};

use crate::{
	connection::{Settings, SingleConnection},
	error::RconError::{self, BusyReconnecting, IO},
	reconnect::Status::{Connected, Disconnected, Stopped},
};

enum Status {
	Connected(SingleConnection),
	Disconnected(String),
	Stopped,
}

struct Internal {
	status: Mutex<Status>,
	close_connection: Notify,
}

/// Drop-in replacement wrapper of [`Connection`](struct.Connection.html) which intercepts all [`IO errors`](enum.Error.html#variant.IO)
/// returned by [`Connection::exec`](struct.Connection.html#method.exec) to start the reconnection thread, and will opt to return [`BusyReconnecting`](enum.Error.html#variant.BusyReconnecting)
/// instead.
///
/// For further docs, refer to [`Connection`](struct.Connection.html), as it shares the same API.
pub struct ReconnectingConnection {
	address: String,
	pass: String,
	settings: Settings,

	internal: Arc<Internal>,
	reconnect_loop: Option<JoinHandle<()>>,
}

impl ReconnectingConnection {
	/// This function behaves identical to [`Connection::open`](struct.Connection.html#method.open).
	pub async fn open(address: impl ToString, pass: impl ToString, settings: Settings) -> Result<Self, RconError> {
		let address = address.to_string();
		let pass = pass.to_string();
		let status = Mutex::new(Connected(
			SingleConnection::open(address.clone(), pass.clone(), settings.clone()).await?,
		));
		let internal = Arc::new(Internal {
			status,
			close_connection: Notify::new(),
		});
		Ok(ReconnectingConnection {
			address,
			pass,
			settings,
			internal,
			reconnect_loop: None,
		})
	}

	/// This function behaves identical to [`Connection::exec`](struct.Connection.html#method.exec) unless `Err([IO](enum.Error.html#variant.IO))` is returned,
	/// in which case it will start reconnecting and return [`BusyReconnecting`](enum.Error.html#variant.BusyReconnecting) until the connection has been re-established.
	pub async fn exec(&mut self, cmd: impl ToString) -> Result<String, RconError> {
		// First, we check if we are actively reconnecting, this must be done within a Mutex
		let result = {
			let mut lock = self.internal.status.lock().await;
			let connection = match lock.deref_mut() {
				Connected(ref mut c) => c,
				Disconnected(msg) => return Err(BusyReconnecting(msg.clone())),
				Stopped => unreachable!("should only set Stopped state when closing connection"),
			};

			// If we are connected, send the request
			connection.exec(cmd).await
		};

		// If the result is an IO error, trigger reconnection and return BusyReconnecting
		if let Err(IO(_)) = result {
			return Err(self.start_reconnect(result.unwrap_err()).await);
		}

		result
	}

	/// Closes the connection, joining any background tasks that were spawned to help manage it.
	pub async fn close(mut self) {
		{
			let mut lock = self.internal.status.lock().await;
			if let Connected(connection) = mem::replace(&mut *lock, Status::Stopped) {
				connection.close().await;
			}
		}

		self.internal.close_connection.notify();
		if let Some(handle) = self.reconnect_loop.take() {
			handle.await.unwrap_or_else(|e| match e.is_cancelled() {
				true => (), // Cancellation is fine.
				false => panic_any(e.into_panic()),
			});
		}
	}

	async fn start_reconnect(&mut self, e: RconError) -> RconError {
		// First, we change the status, which automatically disconnects the old connection
		{
			let mut lock = self.internal.status.lock().await;
			*lock = Disconnected(e.to_string());
		}

		self.reconnect_loop = Some(tokio::spawn(Self::reconnect_loop(
			self.address.clone(),
			self.pass.clone(),
			self.settings.clone(),
			self.internal.clone(),
		)));

		BusyReconnecting(e.to_string())
	}

	async fn reconnect_loop(address: String, pass: String, settings: Settings, internal: Arc<Internal>) {
		loop {
			let close_connection = internal.close_connection.notified();
			let connection = SingleConnection::open(address.clone(), pass.clone(), settings.clone());
			select! {
				Ok(c) = connection => {
					let mut lock = internal.status.lock().await;
					match *lock {
						Stopped => c.close().await,
						_ => {
							*lock = Connected(c);
						}
					}
					return;
				},
				_ = close_connection => return,
				else => (), // Connection error
			};
			let close_connection = internal.close_connection.notified();
			select! {
				_ = delay_for(Duration::from_secs(1)) => (),
				_ = close_connection => return,
			};
		}
	}
}
