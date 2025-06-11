#![allow(dead_code)]

//! Resource table
//!
//! The resource table data contains a series of additional chunks:
//!     * A `ResTable` containing the chunk header and the number of packages values.
//!     * One or more `ResTablePackage` chunks.
//!
//! Specific entries within a resource table can be uniquely identified
//! with a single integer as defined by the ResTable_ref structure.

use crate::{
    chunks::{
        chunk_header::ChunkHeader,
        chunk_types::ChunkType,
        string_pool::StringPool
    },
    errors::AxmlError
};

use std::io::Cursor;

use byteorder::{
    LittleEndian,
    ReadBytesExt
};
use std::io::Read; // Added for read_exact

/// Header for a resource table
#[derive(Debug, PartialEq)]
pub struct ResTable {
    /// Chunk header
    pub header: ChunkHeader,

    /// The number of ResTable_package structures
    pub package_count: u32,

    /// The global string pool for resource values, if present.
    pub global_string_pool: Option<Vec<String>>,

    /// The packages contained in this resource table.
    pub packages: Vec<ResTablePackage>,
}

impl ResTable {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let res_table_chunk_start_pos = axml_buff.position();
        let res_table_header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableType)?;
        let package_count_val = axml_buff.read_u32::<LittleEndian>()?;
        let mut parsed_global_string_pool: Option<Vec<String>> = None;
        let mut parsed_packages = Vec::with_capacity(package_count_val as usize);
        let res_table_content_end_offset = res_table_chunk_start_pos + res_table_header.chunk_size as u64;

        if axml_buff.position() < res_table_content_end_offset && (res_table_content_end_offset - axml_buff.position()) >= 8 {
            let pos_before_peek = axml_buff.position();
            let next_chunk_type = ChunkType::parse_block_type(axml_buff)?;

            if next_chunk_type == ChunkType::ResStringPoolType {
                let mut strings = Vec::new();
                StringPool::from_buff(axml_buff, &mut strings)?;
                parsed_global_string_pool = Some(strings);
            } else {
                axml_buff.set_position(pos_before_peek);
            }
        }

        while parsed_packages.len() < package_count_val as usize &&
              axml_buff.position() < res_table_content_end_offset &&
              (res_table_content_end_offset - axml_buff.position()) >= 8
        {
            let pos_before_package_parse = axml_buff.position();
            let block_type = ChunkType::parse_block_type(axml_buff)?;
            match block_type {
                ChunkType::ResTablePackageType => {
                    let package = ResTablePackage::parse(axml_buff)?;
                    parsed_packages.push(package);
                },
                _ => {
                    axml_buff.set_position(pos_before_package_parse);
                    eprintln!("Warning: Unexpected chunk type {:04X} at offset {} while expecting ResTablePackageType. Parsed {}/{} packages.",
                              block_type as u16, pos_before_package_parse, parsed_packages.len(), package_count_val);
                    break;
                }
            }
        }

        axml_buff.set_position(res_table_content_end_offset);

        Ok(Self {
            header: res_table_header,
            package_count: package_count_val,
            global_string_pool: parsed_global_string_pool,
            packages: parsed_packages,
        })
    }
}

/// Chunk for a resource type
#[derive(Debug, PartialEq)]
pub struct ResTableType {
    /// Chunk header
    pub header: ChunkHeader,
    /// The type identifier this chunk is holding.  Type IDs start at 1.
    pub id: u8,
    /// Must be 0.
    pub flags: u8,
    /// Must be 0.
    pub reserved: u16,
    /// Number of uint32_t entry configuration masks that follow.
    pub entry_count: u32,
    /// Offset from header where ResTable_entry data starts.
    pub entries_start: u32,
    /// Configuration this collection of entries is designed for.
    pub config: ResTableConfig,
    /// The array of offsets to resource entries.
    pub entry_offsets: Vec<u32>,
    /// Parsed resource entries.
    pub entries: Vec<Option<crate::chunks::res_table_entry::ResTableEntry>>,
}

impl ResTableType {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        const NO_ENTRY: u32 = 0xFFFFFFFF;

        let type_chunk_header_start_pos = axml_buff.position() - 2;

        axml_buff.set_position(type_chunk_header_start_pos);
        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableTypeType)?;

        let id = axml_buff.read_u8()?;
        let flags_byte = axml_buff.read_u8()?;
        let reserved = axml_buff.read_u16::<LittleEndian>()?;
        let entry_count_val = axml_buff.read_u32::<LittleEndian>()?;
        let entries_start_val = axml_buff.read_u32::<LittleEndian>()?;

        let config = ResTableConfig::parse(axml_buff)?;

        let mut entry_offsets_vec = Vec::with_capacity(entry_count_val as usize);
        for _ in 0..entry_count_val {
            entry_offsets_vec.push(axml_buff.read_u32::<LittleEndian>()?);
        }

        // let pos_after_offsets_array = axml_buff.position(); // This variable is unused.
        let mut parsed_entries = Vec::with_capacity(entry_count_val as usize);

        let absolute_base_of_entry_data = type_chunk_header_start_pos + entries_start_val as u64;

        for i in 0..(entry_count_val as usize) {
            let entry_offset_within_data_area = entry_offsets_vec[i];
            if entry_offset_within_data_area == NO_ENTRY {
                parsed_entries.push(None);
            } else {
                let absolute_entry_location = absolute_base_of_entry_data + entry_offset_within_data_area as u64;
                axml_buff.set_position(absolute_entry_location);
                let entry = crate::chunks::res_table_entry::ResTableEntry::parse(axml_buff)?;
                parsed_entries.push(Some(entry));
            }
        }

        axml_buff.set_position(type_chunk_header_start_pos + header.chunk_size as u64);

        Ok(ResTableType {
            header,
            id,
            flags: flags_byte,
            reserved,
            entry_count: entry_count_val,
            entries_start: entries_start_val,
            config,
            entry_offsets: entry_offsets_vec,
            entries: parsed_entries,
        })
    }
}

/// Chunk for a resource type specification
#[derive(Debug, PartialEq)]
pub struct ResTableTypeSpec {
    /// Chunk header
    pub header: ChunkHeader,
    /// The type identifier this chunk is holding.  Type IDs start at 1.
    pub id: u8,
    /// Must be 0.
    pub res0: u8,
    /// Must be 0.
    pub res1: u16,
    /// Number of uint32_t entry configuration masks that follow.
    pub entry_count: u32,
    /// The array of configuration masks.
    pub entry_flags: Vec<u32>,
}

impl ResTableTypeSpec {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableTypeSpecType)?;
        let id = axml_buff.read_u8()?;
        let res0 = axml_buff.read_u8()?;
        let res1 = axml_buff.read_u16::<LittleEndian>()?;
        let entry_count = axml_buff.read_u32::<LittleEndian>()?;

        let mut entry_flags = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entry_flags.push(axml_buff.read_u32::<LittleEndian>()?);
        }

        Ok(ResTableTypeSpec {
            header,
            id,
            res0,
            res1,
            entry_count,
            entry_flags,
        })
    }
}

/// A collection of resource data types within a package.  Followed by
/// one or more ResTable_type and ResTable_typeSpec structures containing the
/// entry values for each resource type.
#[derive(Debug, PartialEq)]
pub struct ResTablePackage {
    /// Package header
    pub header: ChunkHeader,

    /// If this is a base package, its ID.  Package IDs
    /// at 1 (corresponding to the value of the package bits in a
    /// resource identifier).  0 means this is not a base package.
    pub id: u32,

    /// Actual name of this package, \0-terminated.
    pub name: [u16; 128],

    /// Offset to a ResStringPool_header defining the resource
    /// type symbol table.  If zero, this package is inheriting from
    /// another base package (overriding specific values in it).
    pub type_strings_offset: u32,

    /// Last index into typeStrings that is for public use by others.
    pub last_public_type: u32,

    /// Offset to a ResStringPool_header defining the resource key
    /// symbol table.  If zero, this package is inheriting from another
    // base package (overriding specific values in it).
    pub key_strings_offset: u32,

    /// Last index into keyStrings that is for public use by others.
    pub last_public_key: u32,

    /// Type ID offset
    pub type_id_offset: u32,

    /// Parsed string pool for type names
    pub type_string_pool: Option<Vec<String>>,

    /// Parsed string pool for key names
    pub key_string_pool: Option<Vec<String>>,

    /// Vector of type specifications
    pub type_specs: Vec<ResTableTypeSpec>,

    /// Vector of types
    pub types: Vec<ResTableType>,
}

impl ResTablePackage {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let current_pos = axml_buff.position();
        let package_chunk_start_pos = current_pos - 2;
        axml_buff.set_position(package_chunk_start_pos);

        let package_header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTablePackageType)?;

        let id = axml_buff.read_u32::<LittleEndian>()?;

        let mut name_bytes: [u16; 128] = [0; 128];
        for i in 0..128 {
            name_bytes[i] = axml_buff.read_u16::<LittleEndian>()?;
        }

        let type_strings_offset_val = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_type = axml_buff.read_u32::<LittleEndian>()?;
        let key_strings_offset_val = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_key = axml_buff.read_u32::<LittleEndian>()?;
        let type_id_offset = axml_buff.read_u32::<LittleEndian>()?;

        let current_pos_after_header_fields = axml_buff.position();

        let mut type_string_pool = None;
        if type_strings_offset_val != 0 {
            let type_pool_abs_offset = package_chunk_start_pos + type_strings_offset_val as u64;
            axml_buff.set_position(type_pool_abs_offset);

            let _type_pool_chunk_type = ChunkType::parse_block_type(axml_buff)?;
            if _type_pool_chunk_type != ChunkType::ResStringPoolType {
                return Err(AxmlError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Expected ResStringPoolType for type strings, found {:?}", _type_pool_chunk_type)
                )));
            }
            let mut strings = Vec::new();
            StringPool::from_buff(axml_buff, &mut strings)?;
            type_string_pool = Some(strings);
        }
        axml_buff.set_position(current_pos_after_header_fields);

        let mut key_string_pool = None;
        if key_strings_offset_val != 0 {
            let key_pool_abs_offset = package_chunk_start_pos + key_strings_offset_val as u64;
            axml_buff.set_position(key_pool_abs_offset);
            let _key_pool_chunk_type = ChunkType::parse_block_type(axml_buff)?;
             if _key_pool_chunk_type != ChunkType::ResStringPoolType {
                return Err(AxmlError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Expected ResStringPoolType for key strings, found {:?}", _key_pool_chunk_type)
                )));
            }
            let mut strings = Vec::new();
            StringPool::from_buff(axml_buff, &mut strings)?;
            key_string_pool = Some(strings);
        }

        axml_buff.set_position(current_pos_after_header_fields);

        let mut type_specs = Vec::new();
        let mut types = Vec::new();

        let package_content_end_offset = package_chunk_start_pos + package_header.chunk_size as u64;

        while axml_buff.position() < package_content_end_offset {
            if package_content_end_offset - axml_buff.position() < 8 {
                break;
            }
            let chunk_read_start_pos = axml_buff.position();
            let chunk_type = ChunkType::parse_block_type(axml_buff)?;

            match chunk_type {
                ChunkType::ResTableTypeSpecType => {
                    let type_spec = ResTableTypeSpec::parse(axml_buff)?;
                    type_specs.push(type_spec);
                }
                ChunkType::ResTableTypeType => {
                    let type_entry = ResTableType::parse(axml_buff)?;
                    types.push(type_entry);
                }
                _ => {
                    let _unknown_header_hsize = axml_buff.read_u16::<LittleEndian>()?;
                    let unknown_chunk_csize = axml_buff.read_u32::<LittleEndian>()?;

                    let bytes_already_read_of_header = 8;

                    if unknown_chunk_csize > bytes_already_read_of_header {
                        let bytes_to_skip = unknown_chunk_csize - bytes_already_read_of_header;
                        if bytes_to_skip > 0 {
                            if axml_buff.position() + bytes_to_skip as u64 > axml_buff.get_ref().len() as u64 {
                                return Err(AxmlError::IoError(std::io::Error::new(
                                    std::io::ErrorKind::UnexpectedEof,
                                    format!("Declared chunk size {} for unknown chunk type {:?} at offset {} would read past EOF", unknown_chunk_csize, chunk_type, chunk_read_start_pos)
                                )));
                            }
                            let mut skip_buf = vec![0; bytes_to_skip as usize];
                            axml_buff.read_exact(&mut skip_buf)?;
                        }
                    } else if unknown_chunk_csize < bytes_already_read_of_header && unknown_chunk_csize !=0 {
                         return Err(AxmlError::IoError(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Declared chunk size {} for unknown chunk type {:?} at offset {} is smaller than header bytes already read ({})", unknown_chunk_csize, chunk_type, chunk_read_start_pos, bytes_already_read_of_header)
                        )));
                    }

                    eprintln!("Skipped unknown chunk type {:?} of size {} at offset {}", chunk_type, unknown_chunk_csize, chunk_read_start_pos);
                }
            }
        }

        Ok(ResTablePackage {
            header: package_header,
            id,
            name: name_bytes,
            type_strings_offset: type_strings_offset_val,
            last_public_type,
            key_strings_offset: key_strings_offset_val,
            last_public_key,
            type_id_offset,
            type_string_pool,
            key_string_pool,
            type_specs,
            types,
        })
    }
}

/// Describes a particular resource configuration.
#[allow(dead_code)]
#[derive(Debug, Default, PartialEq)]
pub struct ResTableConfig {
    pub size: u32,
    pub mcc: u16,
    pub mnc: u16,
    pub language: [u8; 2],
    pub country: [u8; 2],
    pub orientation: u8,
    pub touchscreen: u8,
    pub density: u16,
    pub keyboard: u8,
    pub navigation: u8,
    pub input_flags: u8,
    pub input_pad0: u8,
    pub screen_width: u16,
    pub screen_height: u16,
    pub sdk_version: u16,
    pub minor_version: u16,
    pub screen_layout: u8,
    pub ui_mode: u8,
    pub smallest_screen_width_dp: u16,
    pub screen_width_dp: u16,
    pub screen_height_dp: u16,
    pub locale_script: [u8; 4],
    pub locale_variant: [u8; 8],
    pub screen_layout2: u8,
    pub screen_pad2: u8,
    pub screen_pad3: u16,
}

impl ResTableConfig {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let mut config = ResTableConfig::default();
        let initial_cursor_pos = axml_buff.position();

        config.size = axml_buff.read_u32::<LittleEndian>()?;

        if config.size >= 4 + 0 + 2 {
            config.mcc = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 2 + 2 {
            config.mnc = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 4 + 2 {
            axml_buff.read_exact(&mut config.language)?;
        }
        if config.size >= 4 + 6 + 2 {
            axml_buff.read_exact(&mut config.country)?;
        }
        if config.size >= 4 + 8 + 1 {
            config.orientation = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 9 + 1 {
            config.touchscreen = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 10 + 2 {
            config.density = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 12 + 1 {
            config.keyboard = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 13 + 1 {
            config.navigation = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 14 + 1 {
            config.input_flags = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 15 + 1 {
            config.input_pad0 = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 16 + 2 {
            config.screen_width = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 18 + 2 {
            config.screen_height = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 20 + 2 {
            config.sdk_version = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 4 + 22 + 2 {
            config.minor_version = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 28 && config.size >= 4 + 24 + 1 {
            config.screen_layout = axml_buff.read_u8()?;
        }
        if config.size >= 28 && config.size >= 4 + 25 + 1 {
             config.ui_mode = axml_buff.read_u8()?;
        }
        if config.size >= 32 && config.size >= 4 + 26 + 2 {
            config.smallest_screen_width_dp = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 36 && config.size >= 4 + 28 + 2 {
            config.screen_width_dp = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 36 && config.size >= 4 + 30 + 2 {
            config.screen_height_dp = axml_buff.read_u16::<LittleEndian>()?;
        }
        if config.size >= 40 && config.size >= 4 + 32 + 4 {
            axml_buff.read_exact(&mut config.locale_script)?;
        }
        if config.size >= 48 && config.size >= 4 + 36 + 8 {
            axml_buff.read_exact(&mut config.locale_variant)?;
        }
        if config.size >= 4 + 44 + 1 {
            config.screen_layout2 = axml_buff.read_u8()?;
        }
        if config.size >= 4 + 45 + 1 {
             config.screen_pad2 = axml_buff.read_u8()?;
        }
         if config.size >= 4 + 46 + 2 {
            config.screen_pad3 = axml_buff.read_u16::<LittleEndian>()?;
        }

        axml_buff.set_position(initial_cursor_pos + config.size as u64);

        Ok(config)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::errors::AxmlError;
    use byteorder::{LittleEndian, WriteBytesExt, WriteExt}; // Added WriteExt for write_all

    #[test]
    fn test_parse_config_minimal_size() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(4).unwrap();

        let mut cursor = Cursor::new(data);
        let config = ResTableConfig::parse(&mut cursor)?;

        assert_eq!(config.size, 4);
        assert_eq!(config.mcc, 0);
        assert_eq!(config.mnc, 0);
        assert_eq!(config.language, [0, 0]);
        assert_eq!(config.country, [0, 0]);
        assert_eq!(config.orientation, 0);
        assert_eq!(config.touchscreen, 0);
        assert_eq!(config.density, 0);
        assert_eq!(config.keyboard, 0);
        assert_eq!(config.navigation, 0);
        assert_eq!(config.input_flags, 0);
        assert_eq!(config.input_pad0, 0);
        assert_eq!(config.screen_width, 0);
        assert_eq!(config.screen_height, 0);
        assert_eq!(config.sdk_version, 0);
        assert_eq!(config.minor_version, 0);
        assert_eq!(config.screen_layout, 0);
        assert_eq!(config.ui_mode, 0);
        assert_eq!(config.smallest_screen_width_dp, 0);
        assert_eq!(config.screen_width_dp, 0);
        assert_eq!(config.screen_height_dp, 0);
        assert_eq!(config.locale_script, [0, 0, 0, 0]);
        assert_eq!(config.locale_variant, [0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(config.screen_layout2, 0);
        assert_eq!(config.screen_pad2, 0);
        assert_eq!(config.screen_pad3, 0);

        Ok(())
    }

    #[test]
    fn test_parse_config_size_36_basic_values() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(36).unwrap();
        data.write_u16::<LittleEndian>(120).unwrap();
        data.write_u16::<LittleEndian>(240).unwrap();
        data.write_all(b"en").unwrap();
        data.write_all(b"US").unwrap();
        data.write_u8(1).unwrap();
        data.write_u8(2).unwrap();
        data.write_u16::<LittleEndian>(480).unwrap();
        data.write_u8(1).unwrap();
        data.write_u8(2).unwrap();
        data.write_u8(1).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(1920).unwrap();
        data.write_u16::<LittleEndian>(1080).unwrap();
        data.write_u16::<LittleEndian>(30).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();

        data.write_u8(0x02 | 0x20).unwrap();
        data.write_u8(0x01 | 0x00).unwrap();
        data.write_u16::<LittleEndian>(360).unwrap();

        data.write_u16::<LittleEndian>(380).unwrap();
        data.write_u16::<LittleEndian>(420).unwrap();


        assert_eq!(data.len(), 4 + 32, "Data written does not match expected payload size for config.size=36");

        let mut cursor = Cursor::new(data);
        let config = ResTableConfig::parse(&mut cursor)?;

        assert_eq!(config.size, 36);
        assert_eq!(config.mcc, 120);
        assert_eq!(config.mnc, 240);
        assert_eq!(config.language, [b'e', b'n']);
        assert_eq!(config.country, [b'U', b'S']);
        assert_eq!(config.orientation, 1);
        assert_eq!(config.touchscreen, 2);
        assert_eq!(config.density, 480);
        assert_eq!(config.keyboard, 1);
        assert_eq!(config.navigation, 2);
        assert_eq!(config.input_flags, 1);
        assert_eq!(config.input_pad0, 0);
        assert_eq!(config.screen_width, 1920);
        assert_eq!(config.screen_height, 1080);
        assert_eq!(config.sdk_version, 30);
        assert_eq!(config.minor_version, 0);
        assert_eq!(config.screen_layout, (0x02 | 0x20));
        assert_eq!(config.ui_mode, (0x01 | 0x00));
        assert_eq!(config.smallest_screen_width_dp, 360);
        assert_eq!(config.screen_width_dp, 380);
        assert_eq!(config.screen_height_dp, 420);

        assert_eq!(config.locale_script, [0,0,0,0]);
        assert_eq!(config.locale_variant, [0,0,0,0,0,0,0,0]);
        assert_eq!(config.screen_layout2, 0);
        assert_eq!(config.screen_pad2, 0);
        assert_eq!(config.screen_pad3, 0);

        assert_eq!(cursor.position(), config.size as u64, "Cursor not positioned at the end of config.size");

        Ok(())
    }

    #[test]
    fn test_parse_config_full_size_52() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let config_size = 52u32;
        data.write_u32::<LittleEndian>(config_size).unwrap();

        data.write_u16::<LittleEndian>(1).unwrap();
        data.write_u16::<LittleEndian>(2).unwrap();
        data.write_all(b"ja").unwrap();
        data.write_all(b"JP").unwrap();
        data.write_u8(3).unwrap();
        data.write_u8(4).unwrap();
        data.write_u16::<LittleEndian>(320).unwrap();
        data.write_u8(5).unwrap();
        data.write_u8(6).unwrap();
        data.write_u8(7).unwrap();
        data.write_u8(8).unwrap();
        data.write_u16::<LittleEndian>(800).unwrap();
        data.write_u16::<LittleEndian>(600).unwrap();
        data.write_u16::<LittleEndian>(28).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u8(9).unwrap();
        data.write_u8(10).unwrap();
        data.write_u16::<LittleEndian>(320).unwrap();
        data.write_u16::<LittleEndian>(600).unwrap();
        data.write_u16::<LittleEndian>(800).unwrap();
        data.write_all(b"Latn").unwrap();
        data.write_all(&[b'V', b'A', b'R', b'I', b'A', b'N', b'T', 0]).unwrap();
        data.write_u8(11).unwrap();
        data.write_u8(12).unwrap();
        data.write_u16::<LittleEndian>(13).unwrap();

        assert_eq!(data.len() - 4, config_size as usize - 4, "Payload size mismatch");

        let initial_cursor_pos = 0;
        let mut cursor = Cursor::new(data);
        let config = ResTableConfig::parse(&mut cursor)?;

        assert_eq!(config.size, config_size);
        assert_eq!(config.mcc, 1);
        assert_eq!(config.mnc, 2);
        assert_eq!(config.language, [b'j', b'a']);
        assert_eq!(config.country, [b'J', b'P']);
        assert_eq!(config.orientation, 3);
        assert_eq!(config.touchscreen, 4);
        assert_eq!(config.density, 320);
        assert_eq!(config.keyboard, 5);
        assert_eq!(config.navigation, 6);
        assert_eq!(config.input_flags, 7);
        assert_eq!(config.input_pad0, 8);
        assert_eq!(config.screen_width, 800);
        assert_eq!(config.screen_height, 600);
        assert_eq!(config.sdk_version, 28);
        assert_eq!(config.minor_version, 0);
        assert_eq!(config.screen_layout, 9);
        assert_eq!(config.ui_mode, 10);
        assert_eq!(config.smallest_screen_width_dp, 320);
        assert_eq!(config.screen_width_dp, 600);
        assert_eq!(config.screen_height_dp, 800);
        assert_eq!(config.locale_script, [b'L', b'a', b't', b'n']);
        assert_eq!(config.locale_variant, [b'V', b'A', b'R', b'I', b'A', b'N', b'T', 0]);
        assert_eq!(config.screen_layout2, 11);
        assert_eq!(config.screen_pad2, 12);
        assert_eq!(config.screen_pad3, 13);

        assert_eq!(cursor.position(), initial_cursor_pos + config_size as u64);
        Ok(())
    }

    #[test]
    fn test_parse_config_intermediate_size_28() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let config_size = 28u32;
        data.write_u32::<LittleEndian>(config_size).unwrap();

        data.write_u16::<LittleEndian>(10).unwrap();
        data.write_u16::<LittleEndian>(20).unwrap();
        data.write_all(b"fr").unwrap();
        data.write_all(b"CA").unwrap();
        data.write_u8(1).unwrap();
        data.write_u8(1).unwrap();
        data.write_u16::<LittleEndian>(240).unwrap();
        data.write_u8(2).unwrap();
        data.write_u8(1).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(1280).unwrap();
        data.write_u16::<LittleEndian>(720).unwrap();
        data.write_u16::<LittleEndian>(25).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        assert_eq!(data.len() - 4, config_size as usize - 4, "Payload size mismatch");


        let initial_cursor_pos = 0;
        let mut cursor = Cursor::new(data);
        let config = ResTableConfig::parse(&mut cursor)?;

        assert_eq!(config.size, config_size);
        assert_eq!(config.mcc, 10);
        assert_eq!(config.mnc, 20);
        assert_eq!(config.language, [b'f', b'r']);
        assert_eq!(config.country, [b'C', b'A']);
        assert_eq!(config.orientation, 1);
        assert_eq!(config.touchscreen, 1);
        assert_eq!(config.density, 240);
        assert_eq!(config.keyboard, 2);
        assert_eq!(config.navigation, 1);
        assert_eq!(config.input_flags, 0);
        assert_eq!(config.input_pad0, 0);
        assert_eq!(config.screen_width, 1280);
        assert_eq!(config.screen_height, 720);
        assert_eq!(config.sdk_version, 25);
        assert_eq!(config.minor_version, 0);

        assert_eq!(config.screen_layout, 0);
        assert_eq!(config.ui_mode, 0);
        assert_eq!(config.smallest_screen_width_dp, 0);
        assert_eq!(config.screen_width_dp, 0);
        assert_eq!(config.screen_height_dp, 0);
        assert_eq!(config.locale_script, [0,0,0,0]);
        assert_eq!(config.locale_variant, [0,0,0,0,0,0,0,0]);
        assert_eq!(config.screen_layout2, 0);
        assert_eq!(config.screen_pad2, 0);
        assert_eq!(config.screen_pad3, 0);

        assert_eq!(cursor.position(), initial_cursor_pos + config_size as u64);
        Ok(())
    }

    #[test]
    fn test_parse_config_error_insufficient_data() {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(52).unwrap();
        data.extend_from_slice(&[0u8; 30]);

        let mut cursor = Cursor::new(data);
        let result = ResTableConfig::parse(&mut cursor);

        assert!(matches!(result, Err(AxmlError::IoError(_))), "Expected IoError due to insufficient data for declared size.");
    }

    #[test]
    fn test_parse_config_zero_size() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(0).unwrap();

        let initial_cursor_pos = 0;
        let mut cursor = Cursor::new(data);
        let config = ResTableConfig::parse(&mut cursor)?;

        assert_eq!(config.size, 0);
        let default_config = ResTableConfig::default();
        assert_eq!(config.mcc, default_config.mcc);
        assert_eq!(config.mnc, default_config.mnc);
        assert_eq!(config.language, default_config.language);
        assert_eq!(config.country, default_config.country);
        assert_eq!(config.orientation, default_config.orientation);
        assert_eq!(config.touchscreen, default_config.touchscreen);
        assert_eq!(config.density, default_config.density);
        assert_eq!(config.keyboard, default_config.keyboard);
        assert_eq!(config.navigation, default_config.navigation);
        assert_eq!(config.input_flags, default_config.input_flags);
        assert_eq!(config.input_pad0, default_config.input_pad0);
        assert_eq!(config.screen_width, default_config.screen_width);
        assert_eq!(config.screen_height, default_config.screen_height);
        assert_eq!(config.sdk_version, default_config.sdk_version);
        assert_eq!(config.minor_version, default_config.minor_version);
        assert_eq!(config.screen_layout, default_config.screen_layout);
        assert_eq!(config.ui_mode, default_config.ui_mode);
        assert_eq!(config.smallest_screen_width_dp, default_config.smallest_screen_width_dp);
        assert_eq!(config.screen_width_dp, default_config.screen_width_dp);
        assert_eq!(config.screen_height_dp, default_config.screen_height_dp);
        assert_eq!(config.locale_script, default_config.locale_script);
        assert_eq!(config.locale_variant, default_config.locale_variant);
        assert_eq!(config.screen_layout2, default_config.screen_layout2);
        assert_eq!(config.screen_pad2, default_config.screen_pad2);
        assert_eq!(config.screen_pad3, default_config.screen_pad3);

        assert_eq!(cursor.position(), initial_cursor_pos + config.size as u64, "Cursor position should be initial_pos + 0 for size=0");
        Ok(())
    }

    #[test]
    fn test_parse_type_spec_basic() -> Result<(), AxmlError> {
        let mut test_data = Vec::new();
        test_data.write_u16::<LittleEndian>(ChunkType::ResTableTypeSpecType as u16).unwrap();
        test_data.write_u16::<LittleEndian>(16).unwrap();
        test_data.write_u32::<LittleEndian>(16 + 2 * 4).unwrap();

        test_data.write_u8(1).unwrap();
        test_data.write_u8(0).unwrap();
        test_data.write_u16::<LittleEndian>(0).unwrap();
        test_data.write_u32::<LittleEndian>(2).unwrap();

        test_data.write_u32::<LittleEndian>(0x00000001).unwrap();
        test_data.write_u32::<LittleEndian>(0x40000000).unwrap();

        let mut cursor_for_spec = Cursor::new(test_data);

        let type_spec = ResTableTypeSpec::parse(&mut cursor_for_spec)?;

        assert_eq!(type_spec.header.type_, ChunkType::ResTableTypeSpecType);
        assert_eq!(type_spec.header.header_size, 16);
        assert_eq!(type_spec.header.chunk_size, 24);

        assert_eq!(type_spec.id, 1);
        assert_eq!(type_spec.res0, 0);
        assert_eq!(type_spec.res1, 0);
        assert_eq!(type_spec.entry_count, 2);
        assert_eq!(type_spec.entry_flags, vec![0x00000001, 0x40000000]);

        assert_eq!(cursor_for_spec.position(), 24);

        Ok(())
    }

    #[test]
    fn test_parse_res_table_type_basic() -> Result<(), AxmlError> {
        let mut data = Vec::new();

        let type_specific_header_size: u16 = 8 + 12;

        let res_config_struct_size: u32 = 36;

        let entry_count_val: u32 = 2;
        let entry_offsets_array_size: u32 = entry_count_val * 4;

        let total_chunk_size: u32 = type_specific_header_size as u32 + res_config_struct_size + entry_offsets_array_size;

        data.write_u16::<LittleEndian>(ChunkType::ResTableTypeType as u16).unwrap();
        data.write_u16::<LittleEndian>(type_specific_header_size).unwrap();
        data.write_u32::<LittleEndian>(total_chunk_size).unwrap();

        data.write_u8(1).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u32::<LittleEndian>(entry_count_val).unwrap();

        let entries_start_val_offset: u32 = total_chunk_size;
        data.write_u32::<LittleEndian>(entries_start_val_offset).unwrap();

        data.write_u32::<LittleEndian>(res_config_struct_size).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_all(b"en").unwrap();
        data.write_all(b"US").unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u8(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();
        data.write_u16::<LittleEndian>(0).unwrap();

        data.write_u32::<LittleEndian>(0xFFFFFFFF).unwrap();
        data.write_u32::<LittleEndian>(0xFFFFFFFF).unwrap();

        let mut cursor = Cursor::new(data);

        let res_type = ResTableType::parse(&mut cursor)?;

        assert_eq!(res_type.header.type_, ChunkType::ResTableTypeType);
        assert_eq!(res_type.header.header_size, type_specific_header_size);
        assert_eq!(res_type.header.chunk_size, total_chunk_size);
        assert_eq!(res_type.id, 1);
        assert_eq!(res_type.flags, 0);
        assert_eq!(res_type.entry_count, entry_count_val);
        assert_eq!(res_type.entries_start, entries_start_val_offset);

        assert_eq!(res_type.config.size, res_config_struct_size);
        assert_eq!(res_type.config.language, [b'e', b'n']);
        assert_eq!(res_type.config.country, [b'U', b'S']);

        assert_eq!(res_type.entry_offsets.len(), entry_count_val as usize);
        assert_eq!(res_type.entry_offsets[0], 0xFFFFFFFF);
        assert_eq!(res_type.entry_offsets[1], 0xFFFFFFFF);

        assert_eq!(res_type.entries.len(), entry_count_val as usize);
        assert!(res_type.entries.iter().all(|e| e.is_none()), "All entries should be None for NO_ENTRY offsets");

        assert_eq!(cursor.position(), total_chunk_size as u64, "Cursor not at the end of the ResTable_type chunk");

        Ok(())
    }
}
