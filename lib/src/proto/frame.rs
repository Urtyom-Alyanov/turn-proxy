use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::proto::{HEADER_SIZE, MAGIC_BYTE, command::Command, error::ProtocolError};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct FrameFlags: u16 {
        const FIN           = 0b000_001;
        const COMPRESSED    = 0b000_010;
        const FRAGMENTED    = 0b000_100;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub command: Command,
    pub stream_id: u16,
    pub flags: FrameFlags,
    pub payload: Bytes,
}

impl Frame {
    fn new(command: Command, stream_id: u16, payload: Bytes) -> Self {
        Self {
            command,
            stream_id,
            flags: FrameFlags::empty(),
            payload,
        }
    }

    pub fn with_flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn encode(&self) -> Bytes {
        let payload_len = self.payload.len();
        let mut buf = BytesMut::with_capacity(payload_len + HEADER_SIZE);

        buf.put_u8(MAGIC_BYTE);
        buf.put_u8(self.command as u8);
        buf.put_u16(self.stream_id);
        buf.put_u16(payload_len as u16);
        buf.put_u16(self.flags.bits());
        buf.put_slice(&self.payload);

        buf.freeze()
    }

    pub fn decode(src: &mut Bytes) -> Result<Option<Self>, ProtocolError> {
        if src.len() < HEADER_SIZE {
            return Ok(None);
        }

        if src[0] != MAGIC_BYTE {
            return Err(ProtocolError::MagicIncorrect(src[0]));
        }

        let command = Command::try_from(src[1])?;
        let stream_id = u16::from_be_bytes([src[2], src[3]]);
        let payload_len = u16::from_be_bytes([src[4], src[5]]) as usize;
        let flags_bits = u16::from_be_bytes([src[6], src[7]]);
        let flags = FrameFlags::from_bits_truncate(flags_bits);

        let total_size = HEADER_SIZE + payload_len;

        if src.len() < total_size {
            return Ok(None);
        }

        src.advance(HEADER_SIZE);
        let payload = src.split_to(payload_len);

        Ok(Some(Self {
            command,
            stream_id,
            flags,
            payload,
        }))
    }
}
