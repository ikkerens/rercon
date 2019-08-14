use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::{RconError,
                   RconError::CommandTooLong};

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

    pub(crate) fn read(stream: &mut dyn Read) -> Result<Self, RconError> {
        let len = stream.read_i32::<LittleEndian>()?;
        let mut buf = Vec::with_capacity(len as usize);
        stream.take(len as u64).read_to_end(&mut buf)?;

        let id = (&buf[0..4]).read_i32::<LittleEndian>()?;
        let packet_type = (&buf[4..8]).read_i32::<LittleEndian>()?;
        let body = String::from_utf8((&buf[8..((len - 2) as usize)]).to_vec())?;

        Ok(Packet {
            id,
            packet_type,
            body,
        })
    }

    pub(crate) fn send_internal(&self, stream: &mut dyn Write) -> Result<(), RconError> {
        if self.body.len() > 1014 { // 1024 - 10
            return Err(CommandTooLong);
        }

        stream.write_i32::<LittleEndian>(self.body.len() as i32 + 10)?;
        stream.write_i32::<LittleEndian>(self.id)?;
        stream.write_i32::<LittleEndian>(self.packet_type)?;
        stream.write(self.body.as_bytes())?;
        stream.write_u8(0)?; // null-terminate the string
        stream.write_u8(0)?; // And again, because RCON
        stream.flush()?;

        Ok(())
    }
}
