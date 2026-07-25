use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetAddress {
    Ip(SocketAddr),
    Domain(String, u16),
}

impl TargetAddress {
    pub fn encode(&self, buf: &mut BytesMut) {
        match self {
            Self::Ip(SocketAddr::V4(addr)) => {
                buf.put_u8(0x01);
                buf.put_slice(&addr.ip().octets());
                buf.put_u16(addr.port());
            }
            Self::Ip(SocketAddr::V6(addr)) => {
                buf.put_u8(0x02);
                buf.put_slice(&addr.ip().octets());
                buf.put_u16(addr.port());
            }
            Self::Domain(domain, port) => {
                buf.put_u8(0x03);
                let bytes = domain.as_bytes();
                buf.put_u8(domain.len() as u8);
                buf.put_slice(bytes);
                buf.put_u16(*port);
            }
        }
    }

    pub fn decode(buf: &mut impl Buf) -> Option<Self> {
        if !buf.has_remaining() {
            return None;
        }

        match buf.get_u8() {
            0x01 => {
                if buf.remaining() < 4 + 2 {
                    return None;
                }
                let mut octets = [0u8; 4];
                buf.copy_to_slice(&mut octets);
                let port = buf.get_u16();

                Some(Self::Ip(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::from_octets(octets)),
                    port,
                )))
            }
            0x02 => {
                if buf.remaining() < 16 + 2 {
                    return None;
                }
                let mut octets = [0u8; 16];
                buf.copy_to_slice(&mut octets);
                let port = buf.get_u16();

                Some(Self::Ip(SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::from_octets(octets)),
                    port,
                )))
            }
            0x03 => {
                if !buf.has_remaining() {
                    return None;
                }
                let len = buf.get_u8() as usize;
                if buf.remaining() < len + 2 {
                    return None;
                }
                let mut domain_bytes = vec![0u8; len];
                buf.copy_to_slice(&mut domain_bytes);
                let port = buf.get_u16();

                let domain = String::from_utf8(domain_bytes).ok()?;
                Some(Self::Domain(domain, port))
            }
            _ => None,
        }
    }
}
