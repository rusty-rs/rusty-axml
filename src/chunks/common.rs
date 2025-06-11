//! Contains common structures used across various chunk types.

use crate::errors::AxmlError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// Reference to a resource table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResTable_ref {
    /// The resource identifier.
    pub ident: u32,
}

impl ResTable_ref {
    /// Parses a ResTable_ref from the current position of the AXML buffer.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let ident = axml_buff.read_u32::<LittleEndian>()?;
        Ok(Self { ident })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::errors::AxmlError;
    use byteorder::{LittleEndian, WriteBytesExt};

    #[test]
    fn test_parse_res_table_ref_basic() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let expected_ident: u32 = 0x7F010001; // Example identifier
        data.write_u32::<LittleEndian>(expected_ident).unwrap();

        let mut cursor = Cursor::new(data);
        let res_ref = ResTable_ref::parse(&mut cursor)?;

        assert_eq!(res_ref.ident, expected_ident);
        Ok(())
    }

    #[test]
    fn test_parse_res_table_ref_insufficient_data() {
        let mut data = Vec::new();
        data.write_u16::<LittleEndian>(0x7F01).unwrap(); // Only 2 bytes, need 4

        let mut cursor = Cursor::new(data);
        let result = ResTable_ref::parse(&mut cursor);

        assert!(matches!(result, Err(AxmlError::IoError(_))));
    }
}
