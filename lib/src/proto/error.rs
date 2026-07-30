use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Command `{0:#X}` is not exist's")]
    UnknownCommand(u8),

    #[error("Version `{0:#X} is not exist's or very old")]
    UnknownVersion(u8),

    #[error("Pocket is incorrect! Invalid magic counter {0:#X}")]
    MagicIncorrect(u8),

    #[error("Incorrect pocket header. Incompled")]
    IncompledHeader,

    #[error("Unexpected end of buffer, need at least {0} more bytes")]
    UnexpectedEof(usize),

    #[error("Invalid connection error: {0:#X}")]
    InvalidConnError(u8),

    #[error("Invalid timestamp with millis: {0}")]
    InvalidTimestamp(u64),
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConnError {
    Timeout = 0x01,
    RemoteClosed = 0x02,
    NoRoute = 0x03,
    QuotaExceeded = 0x04,
}

impl TryFrom<u8> for ConnError {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Timeout),
            0x02 => Ok(Self::RemoteClosed),
            0x03 => Ok(Self::NoRoute),
            0x04 => Ok(Self::QuotaExceeded),

            byte => Err(ProtocolError::InvalidConnError(byte)),
        }
    }
}
