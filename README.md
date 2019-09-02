# ReRCON

ReRCON is a RCON library written with primarily the game Ark: Survival Evolved in mind.
While this library is Valve RCON compliant, and should thus work with other RCON applications, it has not been tested for other applications.

There are two primary types to be used with this create,
[`Connection`](https://docs.rs/rercon/1.0.2/rercon/struct.Connection.html) and [`ReConnection`](https://docs.rs/rercon/1.0.2/rercon/struct.ReConnection.html),
both of these types share the same API,
the primary difference is that [`ReConnection::exec`](https://docs.rs/rercon/1.0.2/rercon/struct.ReConnection.html#method.exec) will never return [`IO errors`](https://docs.rs/rercon/1.0.0/rercon/enum.Error.html#variant.IO),
as it will start a new thread to reconnect,
instead, it will return error [`BusyReconnecting`](https://docs.rs/rercon/1.0.2/rercon/enum.Error.html#variant.BusyReconnecting),
with a string being a `to_string` representation of the error that caused the reconnect in the first place.

All public methods use a template to accept all forms of strings that implement `Into<String>`, however the library will always return `std::string::String`

# Example
##### One-off connection:
```rust
use rercon::Connection;

let mut connection = Connection::open("123.456.789.123:27020", "my_secret_password", None).unwrap();
let reply = connection.exec("hello").unwrap();
println!("Reply from server: {}", reply);
```

##### Automatically reconnecting connection:
```rust
use rercon::ReConnection;

let mut connection = ReConnection::open("123.456.789.123:27020", "my_secret_password", None).unwrap();
let reply = connection.exec("hello").unwrap();
println!("Reply from server: {}", reply);
```