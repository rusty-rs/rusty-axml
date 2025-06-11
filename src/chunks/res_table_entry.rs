//! Defines structures related to resource table entries.

use crate::chunks::common::ResTable_ref;
use crate::chunks::res_value::ResValue;
use crate::chunks::string_pool::ResStringPool_ref; // Assuming this will be made public
use crate::errors::AxmlError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

/// A single name/value mapping in a ResTable_map_entry.
#[derive(Debug)]
pub struct ResTable_map {
    pub name: ResTable_ref,
    pub value: ResValue,
}

impl ResTable_map {
    /// Parses a ResTable_map from the current position of the AXML buffer.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let name = ResTable_ref::parse(axml_buff)?;
        let value = ResValue::from_buff(axml_buff)?;
        Ok(Self { name, value })
    }
}

/// Helper structure to represent the content of a complex ResTable_entry (a map).
/// This is not a direct structure from the spec but helps manage the data.
/// Corresponds to ResTable_map_entry in the C++ sources, minus the initial ResTable_entry fields.
#[derive(Debug)]
pub struct ResTable_map_entry_content {
    /// The parent resource identifier of this map.
    pub parent: ResTable_ref,
    /// The number of ResTable_map structures that follow.
    pub count: u32,
    /// The array of ResTable_map structures.
    pub maps: Vec<ResTable_map>,
}

impl ResTable_map_entry_content {
    /// Parses the content of a ResTable_map_entry from the current position of the AXML buffer.
    /// Assumes the initial ResTable_entry fields (size, flags, key) have already been read.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let parent = ResTable_ref::parse(axml_buff)?;
        let count = axml_buff.read_u32::<LittleEndian>()?;
        let mut maps = Vec::with_capacity(count as usize);
        for _ in 0..count {
            maps.push(ResTable_map::parse(axml_buff)?);
        }
        Ok(Self { parent, count, maps })
    }
}

/// Represents a resource entry in the resource table.
/// An entry can be simple (pointing to a Res_value) or complex (a map of name/value pairs).
#[derive(Debug)]
pub struct ResTable_entry {
    /// Total size of this entry (header plus value or map).
    pub size: u16,
    /// Flags associated with this entry.
    pub flags: u16,
    /// Reference to the key string for this entry in the key string pool.
    pub key: ResStringPool_ref, // If ResStringPool_ref is not public, this might need to be u32
    /// The value of this entry, if it's a simple type.
    pub value: Option<ResValue>,
    /// The map of name/value pairs, if this is a complex (map) type.
    pub complex_map: Option<ResTable_map_entry_content>,
}

impl ResTable_entry {
    pub const FLAG_COMPLEX: u16 = 0x0001;
    pub const FLAG_PUBLIC: u16 = 0x0002;
    // Potentially other flags like FLAG_WEAK if needed from ResourceTypes.h

    /// Parses a ResTable_entry from the current position of the AXML buffer.
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let size = axml_buff.read_u16::<LittleEndian>()?;
        let flags = axml_buff.read_u16::<LittleEndian>()?;
        // Assuming ResStringPool_ref can be constructed directly or has a parse method.
        // For now, construct directly as per requirement: "If no parse fn, can be read directly."
        let key_index = axml_buff.read_u32::<LittleEndian>()?;
        let key = ResStringPool_ref { index: key_index };

        let mut value = None;
        let mut complex_map = None;

        if (flags & Self::FLAG_COMPLEX) != 0 {
            // This is a ResTable_map_entry.
            // The ResTable_map_entry specific fields (parent, count) follow directly after the 'key'.
            complex_map = Some(ResTable_map_entry_content::parse(axml_buff)?);
        } else {
            // This is a simple ResTable_entry, followed by a Res_value.
            // The Res_value starts immediately after the 'key'.
            value = Some(ResValue::from_buff(axml_buff)?);
        }
        Ok(Self { size, flags, key, value, complex_map })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunks::common::ResTable_ref;
    use crate::chunks::res_value::{ResValue, DataValueType};
    use crate::chunks::string_pool::ResStringPool_ref;
    use std::io::Cursor;
    use crate::errors::AxmlError;
    use byteorder::{LittleEndian, WriteBytesExt};

    #[test]
    fn test_parse_simple_res_table_entry_string() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let entry_size: u16 = 8; // Size of ResTable_entry's own fields (size, flags, key)
        let value_size: u16 = 8; // Size of a Res_value

        // ResTable_entry fields
        data.write_u16::<LittleEndian>(entry_size).unwrap(); // size of this ResTable_entry part
        data.write_u16::<LittleEndian>(0).unwrap();       // flags = 0 (not complex, not public)
        let key_index: u32 = 10;
        data.write_u32::<LittleEndian>(key_index).unwrap(); // key (ResStringPool_ref index)

        // Res_value fields (following the ResTable_entry)
        data.write_u16::<LittleEndian>(value_size).unwrap(); // Res_value.size
        data.write_u8(0).unwrap();                         // Res_value.res0
        data.write_u8(DataValueType::TypeString as u8).unwrap(); // Res_value.dataType = TYPE_STRING
        let string_pool_index: u32 = 5;
        data.write_u32::<LittleEndian>(string_pool_index).unwrap(); // Res_value.data (index into global string pool)

        let mut cursor = Cursor::new(data);
        let entry = ResTable_entry::parse(&mut cursor)?;

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

        // Cursor should be at the end of the Res_value
        // Total read = entry_size_fields (size,flags,key = 2+2+4=8) + Res_value (8) = 16.
        // However, ResTable_entry.size field refers to its own header part (size, flags, key).
        // The cursor position after ResTable_entry::parse depends on whether it's simple or complex.
        // For a simple entry, it reads ResTable_entry (8 bytes) + Res_value (8 bytes).
        assert_eq!(cursor.position(), (entry_size + value_size) as u64);
        Ok(())
    }

    #[test]
    fn test_parse_complex_res_table_entry_map() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let entry_header_size: u16 = 8; // ResTable_entry part (size, flags, key)

        let map_count: u32 = 1;

        // ResTable_entry fields
        data.write_u16::<LittleEndian>(entry_header_size).unwrap(); // size for the ResTable_entry part
        data.write_u16::<LittleEndian>(ResTable_entry::FLAG_COMPLEX).unwrap(); // flags
        let key_index: u32 = 20;
        data.write_u32::<LittleEndian>(key_index).unwrap(); // key

        // ResTable_map_entry_content fields
        let parent_ident: u32 = 0; // No parent
        data.write_u32::<LittleEndian>(parent_ident).unwrap(); // parent.ident
        data.write_u32::<LittleEndian>(map_count).unwrap();    // count

        // ResTable_map item 1
        let map_name_ident: u32 = 0x01010000; // Example attribute ID
        data.write_u32::<LittleEndian>(map_name_ident).unwrap(); // map.name.ident
        // map.value (Res_value)
        data.write_u16::<LittleEndian>(8).unwrap(); // map.value.size
        data.write_u8(0).unwrap();                  // map.value.res0
        data.write_u8(DataValueType::TypeIntDec as u8).unwrap(); // map.value.dataType
        data.write_u32::<LittleEndian>(12345).unwrap(); // map.value.data

        let mut cursor = Cursor::new(data);
        let entry = ResTable_entry::parse(&mut cursor)?;

        assert_eq!(entry.size, entry_header_size);
        assert_eq!(entry.flags, ResTable_entry::FLAG_COMPLEX);
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

        let map_entry_content_fields_size = 4 + 4; // parent_ident + count
        let single_map_size = 4 + 8; // ResTable_ref (name) + Res_value (value)
        let expected_final_pos = entry_header_size as u64 + map_entry_content_fields_size + (map_count as u64 * single_map_size as u64);
        assert_eq!(cursor.position(), expected_final_pos);

        Ok(())
    }

    #[test]
    fn test_parse_simple_entry_insufficient_data_for_value() {
        let mut data = Vec::new();
        // ResTable_entry fields
        data.write_u16::<LittleEndian>(8).unwrap(); // size
        data.write_u16::<LittleEndian>(0).unwrap(); // flags = 0 (simple)
        data.write_u32::<LittleEndian>(10).unwrap(); // key

        // Partial Res_value (e.g., only 4 bytes when 8 are expected for Res_value)
        data.write_u16::<LittleEndian>(8).unwrap(); // Res_value.size = 8
        data.write_u8(0).unwrap();                 // Res_value.res0
        // Missing dataType and data
        // data.write_u8(DataValueType::TypeString as u8).unwrap();
        // data.write_u32::<LittleEndian>(5).unwrap();

        let mut cursor = Cursor::new(data);
        let result = ResTable_entry::parse(&mut cursor);
        assert!(matches!(result, Err(AxmlError::IoError(_))));
    }

    #[test]
    fn test_parse_complex_entry_insufficient_data_for_map() {
        let mut data = Vec::new();
        // ResTable_entry fields
        data.write_u16::<LittleEndian>(8).unwrap(); // size
        data.write_u16::<LittleEndian>(ResTable_entry::FLAG_COMPLEX).unwrap(); // flags
        data.write_u32::<LittleEndian>(20).unwrap(); // key

        // ResTable_map_entry_content fields
        data.write_u32::<LittleEndian>(0).unwrap(); // parent.ident
        data.write_u32::<LittleEndian>(1).unwrap(); // count = 1 map

        // Partial ResTable_map item (e.g., only 2 bytes for name.ident when 4 are needed)
        data.write_u16::<LittleEndian>(0x0101).unwrap();
        // Missing rest of map.name.ident and the entire map.value

        let mut cursor = Cursor::new(data);
        let result = ResTable_entry::parse(&mut cursor);
        assert!(matches!(result, Err(AxmlError::IoError(_))));
    }
}
