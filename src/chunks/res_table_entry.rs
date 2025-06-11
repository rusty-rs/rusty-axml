//! Defines structures related to resource table entries.

use crate::chunks::common::ResTableRef; // Updated import
use crate::chunks::res_value::ResValue;
use crate::chunks::string_pool::ResStringPoolRef;
use crate::errors::AxmlError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// A single name/value mapping in a ResTable_map_entry.
#[derive(Debug, PartialEq)] // Added PartialEq
pub struct ResTableMap { // Renamed
    pub name: ResTableRef,
    pub value: ResValue,
}

impl ResTableMap { // Renamed
    /// Parses a ResTableMap from the current position of the AXML buffer.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let name = ResTableRef::parse(axml_buff)?;
        let value = ResValue::from_buff(axml_buff)?;
        Ok(Self { name, value })
    }
}

/// Helper structure to represent the content of a complex ResTable_entry (a map).
/// This is not a direct structure from the spec but helps manage the data.
/// Corresponds to ResTable_map_entry in the C++ sources, minus the initial ResTable_entry fields.
#[derive(Debug, PartialEq)] // Added PartialEq
pub struct ResTableMapEntryContent { // Renamed
    /// The parent resource identifier of this map.
    pub parent: ResTableRef,
    /// The number of ResTable_map structures that follow.
    pub count: u32,
    /// The array of ResTable_map structures.
    pub maps: Vec<ResTableMap>, // Updated Vec type
}

impl ResTableMapEntryContent { // Renamed
    /// Parses the content of a ResTable_map_entry from the current position of the AXML buffer.
    /// Assumes the initial ResTable_entry fields (size, flags, key) have already been read.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let parent = ResTableRef::parse(axml_buff)?;
        let count = axml_buff.read_u32::<LittleEndian>()?;
        let mut maps = Vec::with_capacity(count as usize);
        for _ in 0..count {
            maps.push(ResTableMap::parse(axml_buff)?); // Updated usage
        }
        Ok(Self { parent, count, maps })
    }
}

/// Represents a resource entry in the resource table.
/// An entry can be simple (pointing to a Res_value) or complex (a map of name/value pairs).
#[derive(Debug, PartialEq)] // Added PartialEq
pub struct ResTableEntry { // Renamed
    /// Total size of this entry (header plus value or map).
    pub size: u16,
    /// Flags associated with this entry.
    pub flags: u16,
    /// Reference to the key string for this entry in the key string pool.
    pub key: ResStringPoolRef,
    /// The value of this entry, if it's a simple type.
    pub value: Option<ResValue>,
    /// The map of name/value pairs, if this is a complex (map) type.
    pub complex_map: Option<ResTableMapEntryContent>, // Updated type
}

impl ResTableEntry { // Renamed
    pub const FLAG_COMPLEX: u16 = 0x0001;
    pub const FLAG_PUBLIC: u16 = 0x0002;
    // Potentially other flags like FLAG_WEAK if needed from ResourceTypes.h

    /// Parses a ResTable_entry from the current position of the AXML buffer.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let size = axml_buff.read_u16::<LittleEndian>()?;
        let flags = axml_buff.read_u16::<LittleEndian>()?;
        let key_index = axml_buff.read_u32::<LittleEndian>()?;
        let key = ResStringPoolRef { index: key_index };

        let mut value = None;
        let mut complex_map = None;

        if (flags & Self::FLAG_COMPLEX) != 0 {
            complex_map = Some(ResTableMapEntryContent::parse(axml_buff)?); // Updated usage
        } else {
            value = Some(ResValue::from_buff(axml_buff)?);
        }
        Ok(Self { size, flags, key, value, complex_map })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunks::common::ResTableRef;
    use crate::chunks::res_value::{ResValue, DataValueType};
    use crate::chunks::string_pool::ResStringPoolRef;
    use std::io::Cursor;
    use crate::errors::AxmlError;
    use byteorder::{LittleEndian, WriteBytesExt};

    #[test]
    fn test_parse_simple_res_table_entry_string() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let entry_size: u16 = 8;
        let value_size: u16 = 8;

        data.write_u16::<LittleEndian>(entry_size).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        let key_index: u32 = 10;
        data.write_u32::<LittleEndian>(key_index).unwrap();

        data.write_u16::<LittleEndian>(value_size).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(DataValueType::TypeString as u8).unwrap();
        let string_pool_index: u32 = 5;
        data.write_u32::<LittleEndian>(string_pool_index).unwrap();

        let mut cursor = Cursor::new(data);
        let entry = ResTableEntry::parse(&mut cursor)?; // Updated usage

        assert_eq!(entry.size, entry_size);
        assert_eq!(entry.flags, 0);
        assert_eq!(entry.key.index, key_index);
        assert!(entry.complex_map.is_none());
        assert!(entry.value.is_some());

        let res_value = entry.value.unwrap();
        assert_eq!(res_value.size, value_size);
        assert_eq!(res_value.res0, 0);
        assert_eq!(res_value.data_type, DataValueType::TypeString);
        assert_eq!(res_value.data, string_pool_index);

        assert_eq!(cursor.position(), (entry_size + value_size) as u64);
        Ok(())
    }

    #[test]
    fn test_parse_complex_res_table_entry_map() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let entry_header_size: u16 = 8;

        let map_count: u32 = 1;

        data.write_u16::<LittleEndian>(entry_header_size).unwrap();
        data.write_u16::<LittleEndian>(ResTableEntry::FLAG_COMPLEX).unwrap(); // Updated usage
        let key_index: u32 = 20;
        data.write_u32::<LittleEndian>(key_index).unwrap();

        let parent_ident: u32 = 0;
        data.write_u32::<LittleEndian>(parent_ident).unwrap();
        data.write_u32::<LittleEndian>(map_count).unwrap();

        let map_name_ident: u32 = 0x01010000;
        data.write_u32::<LittleEndian>(map_name_ident).unwrap();
        data.write_u16::<LittleEndian>(8).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(DataValueType::TypeIntDec as u8).unwrap();
        data.write_u32::<LittleEndian>(12345).unwrap();

        let mut cursor = Cursor::new(data);
        let entry = ResTableEntry::parse(&mut cursor)?; // Updated usage

        assert_eq!(entry.size, entry_header_size);
        assert_eq!(entry.flags, ResTableEntry::FLAG_COMPLEX); // Updated usage
        assert_eq!(entry.key.index, key_index);
        assert!(entry.value.is_none());
        assert!(entry.complex_map.is_some());

        let map_content = entry.complex_map.unwrap();
        assert_eq!(map_content.parent.ident, parent_ident);
        assert_eq!(map_content.count, map_count);
        assert_eq!(map_content.maps.len(), map_count as usize);

        let map_item = &map_content.maps[0];
        assert_eq!(map_item.name.ident, map_name_ident);
        assert_eq!(map_item.value.data_type, DataValueType::TypeIntDec);
        assert_eq!(map_item.value.data, 12345);

        let map_entry_content_fields_size = 4 + 4;
        let single_map_size = 4 + 8;
        let expected_final_pos = entry_header_size as u64 + map_entry_content_fields_size + (map_count as u64 * single_map_size as u64);
        assert_eq!(cursor.position(), expected_final_pos);

        Ok(())
    }

    #[test]
    fn test_parse_simple_entry_insufficient_data_for_value() {
        let mut data = Vec::new();
        data.write_u16::<LittleEndian>(8).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u32::<LittleEndian>(10).unwrap();

        data.write_u16::<LittleEndian>(8).unwrap();
        data.write_u8(0).unwrap();

        let mut cursor = Cursor::new(data);
        let result = ResTableEntry::parse(&mut cursor); // Updated usage
        assert!(matches!(result, Err(AxmlError::IoError(_))));
    }

    #[test]
    fn test_parse_complex_entry_insufficient_data_for_map() {
        let mut data = Vec::new();
        data.write_u16::<LittleEndian>(8).unwrap();
        data.write_u16::<LittleEndian>(ResTableEntry::FLAG_COMPLEX).unwrap(); // Updated usage
        data.write_u32::<LittleEndian>(20).unwrap();

        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_u32::<LittleEndian>(1).unwrap();

        data.write_u16::<LittleEndian>(0x0101).unwrap();

        let mut cursor = Cursor::new(data);
        let result = ResTableEntry::parse(&mut cursor); // Updated usage
        assert!(matches!(result, Err(AxmlError::IoError(_))));
    }
}
