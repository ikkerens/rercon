use std::io::Write;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::{RconError, RconError::CommandTooLong};

type PacketType = i32;

pub(crate) const TYPE_AUTH: PacketType = 3;
pub(crate) const TYPE_EXEC: PacketType = 2;
pub(crate) const TYPE_RESPONSE: PacketType = 0;
pub(crate) const TYPE_AUTH_RESPONSE: PacketType = 2;

pub(crate) struct Packet {
	id: i32,
	packet_type: PacketType,
	body: String,
}

impl Packet {
	pub(crate) fn new<S: Into<String>>(id: i32, packet_type: PacketType, body: S) -> Self {
		Packet {
			id,
			packet_type,
			body: body.into(),
		}
	}

	pub(crate) fn get_id(&self) -> &i32 {
		&self.id
	}

	pub(crate) fn get_packet_type(&self) -> &PacketType {
		&self.packet_type
	}

	pub(crate) fn get_body(&self) -> &String {
		&self.body
	}

	pub(crate) fn decode_packet_buffer(len: usize, buf: &[u8]) -> Result<Self, RconError> {
		let id = (&buf[0..4]).read_i32::<LittleEndian>()?;
		let packet_type = (&buf[4..8]).read_i32::<LittleEndian>()?;
		let body = String::from_utf8((&buf[8..(len - 2)]).to_vec())?;

		Ok(Packet { id, packet_type, body })
	}

	pub(crate) fn create_packet_buffer(&self) -> Result<Vec<u8>, RconError> {
		if self.body.len() > 1014 {
			// 1024 - 10
			return Err(CommandTooLong);
		}

		let mut buf = Vec::with_capacity(self.body.len() + 14);
		buf.write_i32::<LittleEndian>(self.body.len() as i32 + 10)?;
		buf.write_i32::<LittleEndian>(self.id)?;
		buf.write_i32::<LittleEndian>(self.packet_type)?;
		buf.write_all(self.body.as_bytes())?;
		buf.write_u8(0)?; // null-terminate the string
		buf.write_u8(0)?; // And again, because RCON

		Ok(buf)
	}
}
