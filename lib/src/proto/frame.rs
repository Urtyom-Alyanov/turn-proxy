use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::proto::{HEADER_SIZE, MAGIC_BYTE, command::Command, error::ProtocolError};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct FrameFlags: u16 {
        const FIN           = 0b0000_0000_0000_0001;
        const COMPRESSED    = 0b0000_0000_0000_0010;
        const FRAGMENTED    = 0b0000_0000_0000_0100;
        const URGENT        = 0b0000_0000_0000_1000;
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    First = 0x00,
}

impl TryFrom<u8> for Version {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::First),
            ver => Err(ProtocolError::UnknownVersion(ver)),
        }
    }
}

/// A protocol frame
/// ```
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |   Magic Byte  |    Version    |    Command    |    Reserved   |
/// |     (0x67)    |    (1 byte)   |    (1 byte)   |     (0x00)    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           Stream ID           |       Payload Length (N)      |
/// |           (2 bytes)           |           (2 bytes)           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |       Padding Length (L)      |          Frame Flags          |
/// |           (2 bytes)           |           (2 bytes)           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                       Payload (N bytes)                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                       Padding (L bytes)                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    version: Version,
    command: Command,

    stream_id: u16,
    padding_length: usize,
    payload: Bytes,
    flags: FrameFlags,
}

impl Frame {
    pub fn new(version: Version, command: Command, stream_id: u16, payload: Bytes) -> Self {
        Self {
            version,
            command,
            stream_id,
            payload,
            padding_length: 0usize,
            flags: FrameFlags::empty(),
        }
    }

    pub fn with_flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_padding(mut self, padding_length: usize) -> Self {
        self.padding_length = padding_length;
        self
    }

    pub fn encode(&self, padding_generator: fn(padding_len: usize) -> Bytes) -> Bytes {
        let mut byt =
            BytesMut::with_capacity(HEADER_SIZE + self.padding_length + self.payload.len());

        let payload_length = self.payload.len();
        let padding = padding_generator(self.padding_length);

        // HEADER
        byt.put_u8(MAGIC_BYTE);
        byt.put_u8(self.version as u8);
        byt.put_u8(self.command as u8);
        byt.put_u8(0u8); // Reserved

        byt.put_u16(self.stream_id);
        byt.put_u16(payload_length as u16);

        byt.put_u16(self.padding_length as u16);
        byt.put_u16(self.flags.bits());

        // PAYLOAD
        byt.put_slice(&self.payload);
        byt.put_slice(&padding);

        byt.freeze()
    }

    fn verify_magic(byt: &mut impl Buf) -> Result<(), ProtocolError> {
        let magic = byt.get_u8();
        if magic != MAGIC_BYTE {
            return Err(ProtocolError::MagicIncorrect(magic));
        }
        Ok(())
    }

    /// Decode the frame from buffer
    pub fn decode<BufImpl: Buf>(src: &mut BufImpl) -> Result<Option<Self>, ProtocolError> {
        if !src.has_remaining() {
            return Ok(None);
        }

        if src.remaining() < HEADER_SIZE {
            return Ok(None);
        }

        // HEADER
        Self::verify_magic(src)?;
        let version_byte = src.get_u8();
        let command_byte = src.get_u8();
        let _reserved = src.get_u8();

        let stream_id = src.get_u16();
        let payload_length = src.get_u16() as usize;

        let padding_length = src.get_u16() as usize;
        let flags_bits = src.get_u16();

        let version = Version::try_from(version_byte)?;
        let command = Command::try_from(command_byte)?;
        let flags = FrameFlags::from_bits_retain(flags_bits);

        // PAYLOAD
        if src.remaining() < payload_length + padding_length {
            return Ok(None);
        }
        let payload = src.copy_to_bytes(payload_length);
        let _padding = src.copy_to_bytes(padding_length);

        Ok(Some(Self {
            version,
            command,
            stream_id,
            padding_length,
            payload,
            flags,
        }))
    }
}
