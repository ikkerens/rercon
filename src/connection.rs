use std::{
	io::ErrorKind,
	net::SocketAddr::{self, V4, V6},
	pin::Pin,
	sync::{
		atomic::{AtomicI32, Ordering},
		Arc,
	},
	time::Duration,
};

use tokio::{
	io::AsyncRead,
	net::{
		lookup_host,
		tcp::{OwnedReadHalf, OwnedWriteHalf},
		TcpStream, ToSocketAddrs,
	},
	select,
	sync::{mpsc, Notify},
	time::{delay_for, timeout},
};

use crate::{
	error::RconError::{self, DesynchronizedPacket, PasswordIncorrect, UnexpectedPacket, IO},
	packet::{Packet, TYPE_AUTH, TYPE_AUTH_RESPONSE, TYPE_EXEC, TYPE_RESPONSE},
};

/// Settings struct which can be used to adapt behaviour slightly which might help with nonconformant servers.
#[derive(Clone)]
pub struct Settings {
	/// Maximum time allowed to set up a Tcp connection before giving up with a timeout. The maximum timeout possible
	/// will be this multiplied by the amount of IPs the host resolves to.
	pub connect_timeout: Duration,
	/// Delay inbetween TCP connection establishment and sending of the first (auth) packet, needed for older Minecraft
	/// servers.
	pub auth_delay: Option<Duration>,
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			connect_timeout: Duration::from_secs(10),
			auth_delay: None,
		}
	}
}

/// Represents a single-established RCON connection to the server, which will not automatically reconnect once the connection has failed.
/// This struct will instead opt to return [`IO errors`](enum.Error.html#variant.IO), leaving connection responsibility in the callers hands.
///
/// # Example
/// ```rust,no_run
/// use rercon::{Connection, Settings};
///
/// #[tokio::main]
/// async fn main() {
///     let mut connection = Connection::open("123.456.789.123:27020", "my_secret_password", Settings::default()).await.unwrap();
///     let reply = connection.exec("hello").await.unwrap();
///     println!("Reply from server: {}", reply);
/// }
/// ```
pub struct SingleConnection {
	write: OwnedWriteHalf,
	counter: i32,
	receiver: ReceiverHandle,
}

impl SingleConnection {
	/// Opens a new RCON connection, with an optional timeout, and authenticates the connection to the remote server.
	/// If connect_timeout is set to None, a default timeout of 10 seconds will be used.
	pub async fn open(address: impl ToSocketAddrs, pass: impl ToString, settings: Settings) -> Result<Self, RconError> {
		let stream = try_connect(address, settings.connect_timeout).await?;
		let (mut read, mut write) = stream.into_split();

		if let Some(auth_delay) = settings.auth_delay {
			delay_for(auth_delay).await;
		}

		Packet::new(0, TYPE_AUTH, pass.to_string())
			.send_internal(Pin::new(&mut write))
			.await?;
		{
			let response = Packet::read(Pin::new(&mut read)).await?;
			if response.get_packet_type() != TYPE_AUTH_RESPONSE {
				return Err(UnexpectedPacket);
			}
			if response.get_id() == -1 {
				return Err(PasswordIncorrect);
			}
		}

		let receiver = ReceiverHandle::new(read);

		Ok(Self {
			write,
			counter: 0,
			receiver,
		})
	}

	/// Sends a command to the RCON server, returning the combined reply (in case there are multiple packets) or an error.
	pub async fn exec(&mut self, cmd: impl ToString) -> Result<String, RconError> {
		// Send the original command.
		let original_id = self.next_counter();
		self.receiver.set_request_id(original_id);
		Packet::new(original_id, TYPE_EXEC, cmd.to_string())
			.send_internal(Pin::new(&mut self.write))
			.await?;

		// After the first read, we send an empty command, which should be mirrored.
		// We do this because some RCON servers don't properly respond if we send execs
		// too fast. So we wait for the first response.
		// Our counter can never be negative due to overflow protection.
		self.receiver.wait_for_first_packet().await;
		let end_id = self.next_counter();
		Packet::new(end_id, TYPE_EXEC, "".to_string())
			.send_internal(Pin::new(&mut self.write))
			.await?;

		self.receiver.get_response().await
	}

	fn next_counter(&mut self) -> i32 {
		self.counter = next_counter(self.counter);
		self.counter
	}
}

fn next_counter(counter: i32) -> i32 {
	match counter.checked_add(1) {
		Some(new) => new,
		None => 1,
	}
}

struct ReceiverHandle {
	shared: Arc<ReceiverHandleShared>,
	receiver: mpsc::Receiver<Result<String, RconError>>,
}

impl ReceiverHandle {
	pub fn new(stream: OwnedReadHalf) -> Self {
		let shared = Arc::new(ReceiverHandleShared {
			request_id: AtomicI32::new(-1),
			received_first_response: Notify::new(),
			close_connection: Notify::new(),
		});
		let (sender, receiver) = mpsc::channel(1);
		tokio::spawn(receive_loop(stream, shared.clone(), sender));
		Self { shared, receiver }
	}

	pub fn set_request_id(&mut self, id: i32) {
		self.shared.request_id.store(id, Ordering::Release);
	}

	pub async fn wait_for_first_packet(&mut self) {
		self.shared.received_first_response.notified().await;
	}

	async fn get_response(&mut self) -> Result<String, RconError> {
		match self.receiver.recv().await {
			Some(val) => val,
			None => Err(RconError::IO(std::io::Error::new(
				ErrorKind::ConnectionReset,
				"receiving task terminated unexpectedly",
			))),
		}
	}
}

impl Drop for ReceiverHandle {
	fn drop(&mut self) {
		self.shared.close_connection.notify();
	}
}

struct ReceiverHandleShared {
	request_id: AtomicI32,
	received_first_response: Notify,
	close_connection: Notify,
}

async fn receive_loop(
	mut stream: OwnedReadHalf, shared: Arc<ReceiverHandleShared>, mut sender: mpsc::Sender<Result<String, RconError>>,
) {
	loop {
		let response = receive_response(Pin::new(&mut stream), &shared).await;
				shared.request_id.store(-1, Ordering::Release);
		let _ = sender.send(response).await;
			}
			}

async fn receive_response(
	mut stream: Pin<&mut impl AsyncRead>, shared: &ReceiverHandleShared,
) -> Result<String, RconError> {
	let mut end_id = -1;
	let mut result = String::new();

	// Loop until we have confirmation the message is complete.
	loop {
		// Read the first response to the command.
		let response = select! {
			packet = Packet::read(stream.as_mut()) => packet,
			_ = shared.close_connection.notified() => Err(RconError::IO(std::io::Error::new(ErrorKind::ConnectionReset, "closed listener task"))),
		}?;

		let original_id = shared.request_id.load(Ordering::Acquire); // TODO: can this be loosened?
		if original_id <= 0 {
			// Not currently listening for a response.
			// (SingleConnection always uses a positive counter.)
			return Err(DesynchronizedPacket);
		}

		// Check if we received the correct ID. If not, either the client or server is buggy or non-conformant. We'll skip the packet.
		if response.get_id() != original_id && response.get_id() != end_id {
			continue;
		}

		// We should only be receiving a response at this time.
		if response.get_packet_type() != TYPE_RESPONSE {
			return Err(UnexpectedPacket);
		}

		// Let the sending task send the empty command.
		if end_id == -1 {
			end_id = next_counter(original_id);
			shared.received_first_response.notify();
		}

		// If we receive a response to our empty command, that means all
		// previous messages have been sent and (hopefully) received. That means
		// we can finish up our result.
		if response.get_id() == end_id {
			break;
		}

		// All checks have passed; append body to the end result.
		result += response.get_body();
	}

	Ok(result)
}

async fn try_connect(address: impl ToSocketAddrs, timeout_duration: Duration) -> Result<TcpStream, RconError> {
	// Resolve the host
	let mut addrs: Vec<SocketAddr> = lookup_host(address).await?.collect();
	// Sorted by IPv4 first, as these are more likely to succeed as most RCON implementations only bind to IPv4.
	addrs.sort_by_key(|a| match a {
		V4(_) => 0,
		V6(_) => 1,
	});

	// Attempt connecting to all possible outcomes of the resolve
	let mut error = None;
	for addr in addrs {
		match timeout(timeout_duration, TcpStream::connect(&addr)).await {
			Ok(Ok(stream)) => return Ok(stream),  // Successful connection
			Ok(Err(e)) => error = Some(e.into()), // Connecting failed, store error for later
			Err(_) => continue,                   // Timeout expired
		}
	}

	// So at this point, no connection succeeded. Which means either they errored, or... there was nothing to try.
	Err(error.unwrap_or_else(|| {
		IO(std::io::Error::new(
			ErrorKind::AddrNotAvailable,
			"Could not resolve rcon host addr",
		))
	}))
}
