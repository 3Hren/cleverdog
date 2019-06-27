use core::{
    fmt::{self, Display, Formatter, LowerHex, UpperHex},
    num::ParseIntError,
    str::FromStr,
};
use std::error::Error;

/// An error that can occur during parsing a MAC address string.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Parsing of the MAC address contained an invalid digit.
    InvalidDigit(ParseIntError),
    /// The MAC address did not have the correct length.
    InvalidLength,
}

impl Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ParseError::InvalidDigit(..) => write!(fmt, "invalid digit"),
            ParseError::InvalidLength => fmt.write_str("invalid length"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::InvalidDigit(err) => Some(err),
            ParseError::InvalidLength => None,
        }
    }
}

/// MAC address.
#[derive(Debug, Clone, Copy)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// Constructs a new MAC address from the given fixed array.
    #[inline]
    pub fn new(buf: [u8; 6]) -> Self {
        Self(buf)
    }

    /// Parses the specified string into MAC address.
    ///
    /// ```
    /// use cleverdog::mac::MacAddr;
    ///
    /// let mac: MacAddr = "dc:a9:04:97:9d:9b".parse().unwrap();
    /// assert_eq!(mac.as_bytes(), [220, 169, 4, 151, 157, 155]);
    /// ```
    #[inline]
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        <MacAddr as FromStr>::from_str(s)
    }

    /// Returns the underlying byte array.
    #[inline]
    pub fn as_bytes(&self) -> [u8; 6] {
        match self {
            MacAddr(buf) => *buf,
        }
    }
}

impl FromStr for MacAddr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut buf = [0u8; 6];
        let mut idx = 0;

        for b in s.split(':') {
            if idx == 6 {
                return Err(ParseError::InvalidLength);
            }

            buf[idx] = u8::from_str_radix(b, 16).map_err(|err| ParseError::InvalidDigit(err))?;
            idx += 1;
        }

        if idx != 6 {
            return Err(ParseError::InvalidLength);
        }

        Ok(MacAddr::new(buf))
    }
}

impl Display for MacAddr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        <MacAddr as LowerHex>::fmt(self, fmt)
    }
}

impl LowerHex for MacAddr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{:<02x}", self.as_bytes()[0])?;

        for v in self.as_bytes().iter().skip(1) {
            write!(fmt, ":{:<02x}", v)?;
        }

        Ok(())
    }
}

impl UpperHex for MacAddr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{:<02X}", self.as_bytes()[0])?;

        for v in self.as_bytes().iter().skip(1) {
            write!(fmt, ":{:<02X}", v)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let v = "dc:a9:04:97:9d:9b";
        let mac = v.parse::<MacAddr>().unwrap();
        assert_eq!(mac.as_bytes(), [220, 169, 4, 151, 157, 155]);
    }

    #[test]
    fn test_display() {
        let mac = MacAddr::new([220, 169, 4, 151, 157, 155]);
        assert_eq!(&format!("{}", mac), "dc:a9:04:97:9d:9b");
    }

    #[test]
    fn test_display_lower_hex() {
        let mac = MacAddr::new([220, 169, 4, 151, 157, 155]);
        assert_eq!(&format!("{:x}", mac), "dc:a9:04:97:9d:9b");
    }

    #[test]
    fn test_display_upper_hex() {
        let mac = MacAddr::new([220, 169, 4, 151, 157, 155]);
        assert_eq!(&format!("{:X}", mac), "DC:A9:04:97:9D:9B");
    }
}
