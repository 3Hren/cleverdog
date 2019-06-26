use std::{
    convert::TryInto,
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Clone)]
pub struct BufferTooSmall;

impl Display for BufferTooSmall {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "buffer too small")
    }
}

impl Error for BufferTooSmall {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Header<'a>(&'a [u8]);

impl<'a> Header<'a> {
    pub fn from_slice(buf: &'a [u8]) -> Result<Self, BufferTooSmall> {
        if buf.len() < 12 {
            return Err(BufferTooSmall);
        }

        Ok(Header(&buf[..12]))
    }

    #[inline]
    pub fn version(&self) -> u8 {
        let buf = self.as_slice();
        let byte = buf[0];
        byte >> 6
    }

    #[inline]
    pub fn sequence_number(&self) -> u16 {
        u16::from_be_bytes(self.as_slice()[2..4].try_into().unwrap())
    }

    #[inline]
    pub fn timestamp(&self) -> u32 {
        u32::from_be_bytes(self.as_slice()[4..8].try_into().unwrap())
    }

    #[inline]
    pub fn ssrc(&self) -> u32 {
        u32::from_be_bytes(self.as_slice()[8..12].try_into().unwrap())
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Header(buf) => buf,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_rtp() {
        let header = Header(&[128, 96, 0, 17, 0, 0, 140, 160, 0, 0, 0, 16]);

        assert_eq!(2, header.version());
        assert_eq!(17, header.sequence_number());
        assert_eq!(36000, header.timestamp());
        assert_eq!(16, header.ssrc());
    }
}
