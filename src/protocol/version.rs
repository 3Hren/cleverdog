use core::{num::ParseIntError};
use core::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub struct Version([u16; 4]);

impl Version {
    #[inline]
    pub fn new(buf: [u16; 4]) -> Self {
        Self(buf)
    }

    pub fn from_str(v: &str) -> Result<Self, ParseIntError> {
        let mut buf = [0u16; 4];
        let mut idx = 0;

        for b in v.split('.') {
            if idx == 4 {
                break;
            }

            buf[idx] = u16::from_str_radix(b, 10)?;
            idx += 1;
        }

        Ok(Self::new(buf))
    }
}

impl Display for Version {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        let Version([major, minor, patch, release]) = self;
        write!(fmt, "{}.{}.{}.{}", major, minor, patch, release)
    }
}
