use crate::proto::error::ProtocolError;

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Command {
    // Keep alive commands
    Ping = 0x00,
    Pong = 0x01,
    Authorize = 0x02,
    WindowUpdate = 0x03,

    // TCP commands
    TcpOpen = 0x10,
    TcpOpenResult = 0x11,
    TcpData = 0x12,
    TcpClose = 0x13,

    // UDP Commands
    UdpAssociate = 0x20,
    UdpAssociateResult = 0x21,
    UdpData = 0x22,
    UdpTermanate = 0x23,
}

impl TryFrom<u8> for Command {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Ping),
            0x01 => Ok(Self::Pong),
            0x02 => Ok(Self::Authorize),
            0x03 => Ok(Self::WindowUpdate),

            0x10 => Ok(Self::TcpOpen),
            0x11 => Ok(Self::TcpOpenResult),
            0x12 => Ok(Self::TcpData),
            0x13 => Ok(Self::TcpClose),

            0x20 => Ok(Self::UdpAssociate),
            0x21 => Ok(Self::UdpAssociateResult),
            0x22 => Ok(Self::UdpData),
            0x23 => Ok(Self::UdpTermanate),

            cmd => Err(ProtocolError::UnknownCommand(cmd)),
        }
    }
}
