use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use crate::proto::{
    addr_target::TargetAddress,
    command::Command,
    error::{ConnError, ProtocolError},
    traits::{ProtocolDecode, ProtocolEncode},
};

type OpenResult = Result<(), ConnError>;

pub enum Message {
    // Service commands
    Ping {
        timestamp: DateTime<Utc>,
    },
    Pong {
        timestamp: DateTime<Utc>,
    },
    Authorize {
        fingerprint: [u8; 32],
        session_id: Uuid,
        hmac: [u8; 32],
    },
    WindowUpdate {
        credit: u32,
    },

    // TCP
    TcpOpen {
        target: TargetAddress,
    },
    TcpOpenResult(OpenResult),
    TcpData {
        data: Bytes,
    },
    TcpClose {
        reason: Option<ConnError>,
    },

    // UDP
    UdpAssociate,
    UdpAssociateResult(OpenResult),
    UdpData {
        target: TargetAddress,
        data: Bytes,
    },
    UdpTerminate {
        reason: Option<ConnError>,
    },
}

/// Read a sized array of bytes from `Buf`
fn get_array<const N: usize>(buf: &mut impl Buf) -> Result<[u8; N], ProtocolError> {
    if buf.remaining() < N {
        return Err(ProtocolError::UnexpectedEof(N));
    }
    let mut arr = [0u8; N];
    buf.copy_to_slice(&mut arr);
    Ok(arr)
}

struct TimestampService;

impl TimestampService {
    pub fn decode(buf: &mut impl Buf) -> Result<DateTime<Utc>, ProtocolError> {
        let millis = buf.get_u64();

        match Utc.timestamp_millis_opt(millis as i64) {
            chrono::offset::LocalResult::Single(dt) => Ok(dt),
            _ => Err(ProtocolError::InvalidTimestamp(millis)),
        }
    }

    pub fn encode(dt: &DateTime<Utc>, buf: &mut impl BufMut) {
        buf.put_u64(dt.timestamp_millis() as u64);
    }
}

impl ProtocolDecode for OpenResult {
    fn decode(buf: &mut impl Buf) -> Result<Self, ProtocolError> {
        match buf.get_u8() {
            0x00 => Ok(Ok(())),
            code => Ok(Err(ConnError::try_from(code)?)),
        }
    }
}

impl ProtocolEncode for OpenResult {
    fn encode(&self, buf: &mut BytesMut) {
        let code = match &self {
            Ok(()) => 0x00,
            Err(e) => *e as u8,
        };

        buf.put_u8(code);
    }
}

impl Message {
    pub fn ping_now() -> Self {
        Self::Ping {
            timestamp: Utc::now(),
        }
    }

    pub fn pong_for(ping_ts: DateTime<Utc>) -> Self {
        Self::Pong { timestamp: ping_ts }
    }

    pub fn command(&self) -> Command {
        match self {
            Self::Ping { .. } => Command::Ping,
            Self::Pong { .. } => Command::Pong,
            Self::Authorize { .. } => Command::Authorize,
            Self::WindowUpdate { .. } => Command::WindowUpdate,
            Self::TcpOpen { .. } => Command::TcpOpen,
            Self::TcpOpenResult(_) => Command::TcpOpenResult,
            Self::TcpData { .. } => Command::TcpData,
            Self::TcpClose { .. } => Command::TcpClose,
            Self::UdpAssociate => Command::UdpAssociate,
            Self::UdpAssociateResult(_) => Command::UdpAssociateResult,
            Self::UdpData { .. } => Command::UdpData,
            Self::UdpTerminate { .. } => Command::UdpTerminate,
        }
    }

    pub fn decode(command: Command, payload: &mut impl Buf) -> Result<Self, ProtocolError> {
        Ok(match command {
            Command::Ping => Self::Ping {
                timestamp: TimestampService::decode(payload)?,
            },
            Command::Pong => Self::Pong {
                timestamp: TimestampService::decode(payload)?,
            },
            Command::Authorize => Self::Authorize {
                fingerprint: get_array(payload)?,
                session_id: Uuid::from_bytes(get_array(payload)?),
                hmac: get_array(payload)?,
            },
            Command::WindowUpdate => Self::WindowUpdate {
                credit: u32::from_be_bytes(get_array(payload)?),
            },

            Command::TcpOpen => Self::TcpOpen {
                target: TargetAddress::decode(payload).ok_or(ProtocolError::UnexpectedEof(1))?,
            },
            Command::TcpOpenResult => Self::TcpOpenResult(OpenResult::decode(payload)?),
            Command::TcpData => Self::TcpData {
                data: payload.copy_to_bytes(payload.remaining()),
            },
            Command::TcpClose => Self::TcpClose {
                reason: OpenResult::decode(payload)?.err(),
            },

            Command::UdpAssociate => Self::UdpAssociate,
            Command::UdpAssociateResult => Self::UdpAssociateResult(OpenResult::decode(payload)?),
            Command::UdpData => {
                let target =
                    TargetAddress::decode(payload).ok_or(ProtocolError::UnexpectedEof(1))?;
                let data = payload.copy_to_bytes(payload.remaining());
                Self::UdpData { target, data }
            }
            Command::UdpTerminate => Self::UdpTerminate {
                reason: OpenResult::decode(payload)?.err(),
            },
        })
    }
}

impl ProtocolEncode for Message {
    /// Encode into buffer
    fn encode(&self, buf: &mut BytesMut) {
        match self {
            Self::Ping { timestamp } | Self::Pong { timestamp } => {
                TimestampService::encode(timestamp, buf);
            }
            Self::Authorize {
                fingerprint,
                session_id,
                hmac,
            } => {
                buf.put_slice(fingerprint);
                buf.put_slice(session_id.as_bytes());
                buf.put_slice(hmac);
            }
            Self::WindowUpdate { credit } => buf.put_u32(*credit),

            Self::TcpOpen { target } => target.encode(buf),
            Self::TcpOpenResult(result) => result.encode(buf),
            Self::TcpData { data } => buf.put_slice(data),
            Self::TcpClose { reason } => {
                buf.put_u8(reason.map(|e| e as u8).unwrap_or(0x00));
            }

            Self::UdpAssociate => {}
            Self::UdpAssociateResult(result) => result.encode(buf),
            Self::UdpData { target, data } => {
                target.encode(buf);
                buf.put_slice(data);
            }
            Self::UdpTerminate { reason } => {
                buf.put_u8(reason.map(|e| e as u8).unwrap_or(0x00));
            }
        }
    }
}
