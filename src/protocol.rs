pub use crate::protocol::{
    scan::{LookupInfo, ScanInfo},
    version::Version,
};

mod scan;
mod version;

/// Magic constant that is prepended to each camera frame.
///
/// Represents a big-endian integer representation of `[0x4d, 0x4a]` array.
pub const MAGIC: u16 = 19786;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_magic_endianess() {
        assert_eq!([0x4d, 0x4a], MAGIC.to_be_bytes());
    }
}
