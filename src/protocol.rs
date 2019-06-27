use core::{convert::TryFrom, str};
use std::net::SocketAddr;

use crate::mac::MacAddr;
pub use crate::protocol::version::Version;

mod version;

/// Magic constant that is prepended to each camera frame.
///
/// Represents a big-endian integer representation of `[0x4d, 0x4a]` array.
pub const MAGIC: u16 = 19786;

#[derive(Debug, Clone, Copy)]
pub struct ScanInfo {
    /// Camera MAC address.
    mac: MacAddr,
    /// Firmware version.
    version: Version,
}

impl TryFrom<&[u8]> for ScanInfo {
    type Error = &'static str;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        let mut it = v.split(|&ch| ch == b'\0');

        let mac = match it.next() {
            Some(mac) => match str::from_utf8(mac.into()) {
                Ok(mac) => match MacAddr::from_str(mac) {
                    Ok(mac) => mac,
                    Err(..) => return Err("MAC address is invalid"),
                }
                Err(..) => return Err("MAC address contains invalid UTF-8 sequence"),
            },
            None => return Err("missing MAC address"),
        };

        let version = match it.next() {
            Some(version) => match str::from_utf8(version.into()) {
                Ok(version) => match Version::from_str(version) {
                    Ok(version) => version,
                    Err(..) => return Err("version is invalid"),
                }
                Err(..) => return Err("version contains invalid UTF-8 sequence"),
            },
            None => return Err("missing version"),
        };

        let v = Self { mac, version };

        Ok(v)
    }
}

#[derive(Debug, Clone, Copy)]
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
    pub fn mac(&self) -> &MacAddr {
        &self.info.mac
    }

    /// Returns camera's firmware version.
    #[inline]
    pub fn version(&self) -> &Version {
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
