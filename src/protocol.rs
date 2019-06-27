use core::convert::TryFrom;

use std::{error::Error, net::SocketAddr};

/// Magic constant that is prepended to each camera frame.
///
/// Represents a big-endian integer representation of `[0x4d, 0x4a]` array.
pub const MAGIC: u16 = 19786;

#[derive(Debug)]
pub enum ScanError {
    MissingMacAddress,
    MissingVersion,
}

#[derive(Debug)]
pub struct ScanInfo {
    /// Camera MAC address.
    mac: String,
    /// Firmware version.
    version: String,
}

impl TryFrom<&[u8]> for ScanInfo {
    type Error = Box<dyn Error>;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        let mut it = v.split(|&ch| ch == b'\0');

        let mac = match it.next() {
            Some(mac) => String::from_utf8(mac.into())?,
            None => return Err("missing mac address".into()),
        };

        let version = match it.next() {
            Some(version) => String::from_utf8(version.into())?,
            None => return Err("missing version".into()),
        };

        let v = Self { mac, version };

        Ok(v)
    }
}

#[derive(Debug)]
pub struct LookupInfo {
    /// Camera endpoint.
    addr: SocketAddr,
    /// Camera ID.
    cid: [u8; 16],
    /// Scan info.
    info: ScanInfo,
}

impl LookupInfo {
    pub fn new(addr: SocketAddr, cid: [u8; 16], info: ScanInfo) -> Self {
        Self { addr, cid, info }
    }

    /// Returns socket address where the camera is bound.
    #[inline]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns camera's client id.
    #[inline]
    pub fn cid(&self) -> &[u8] {
        &self.cid[..]
    }

    /// Returns camera's MAC address.
    #[inline]
    pub fn mac(&self) -> &str {
        &self.info.mac
    }

    /// Returns camera's firmware version.
    #[inline]
    pub fn version(&self) -> &str {
        &self.info.version
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_magic_endianess() {
        assert_eq!([0x4d, 0x4a], MAGIC.to_be_bytes());
    }
}
