use crate::proto::error::ProtocolError;

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Command {
    // Keep alive commands
    Ping = 0x01,
    Pong = 0x02,

    // TCP commands
    TcpOpen = 0x10,
    TcpData = 0x11,
    TcpClose = 0x12,

    // UDP Commands
    UdpAssociate = 0x20,
    UdpData = 0x21,
    UdpDetach = 0x22,
}

impl TryFrom<u8> for Command {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Ping),
            0x02 => Ok(Self::Pong),

            0x10 => Ok(Self::TcpOpen),
            0x11 => Ok(Self::TcpData),
            0x12 => Ok(Self::TcpClose),

            0x20 => Ok(Self::UdpAssociate),
            0x21 => Ok(Self::UdpData),
            0x22 => Ok(Self::UdpDetach),

            cmd => Err(ProtocolError::UnknownCommand(cmd)),
        }
    }
}
