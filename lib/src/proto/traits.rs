use bytes::{Buf, BytesMut};

use crate::proto::error::ProtocolError;

pub trait ProtocolDecode: Sized {
    fn decode(buf: &mut impl Buf) -> Result<Self, ProtocolError>;
}

pub trait ProtocolEncode {
    fn encode(&self, buf: &mut BytesMut);
}
