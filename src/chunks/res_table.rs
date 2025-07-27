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

/// Header for a resource table
pub struct ResTable {
    /// Chunk header
    header: ChunkHeader,

    /// The number of ResTable_package structures
    pub package_count: u32,
}

impl ResTable {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        // Go back 2 bytes, to account from the block type
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

        // Parse chunk header
        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableType)?;

        // Get package count
        let package_count = axml_buff.read_u32::<LittleEndian>()?;

        let mut strings = Vec::<String>::new();

        // TODO: these are just ignored
        for _ in 0..package_count {
            let block_type = ChunkType::parse_block_type(axml_buff)?;
            match block_type {
                ChunkType::ResStringPoolType => {
                    StringPool::from_buff(axml_buff, &mut strings)?;
                },
                ChunkType::ResTablePackageType => {
                    ResTablePackage::parse(axml_buff)?;
                },
                _ => { panic!("######## Unexpected block type: {block_type:02X}"); }
            };
        }

        Ok(Self {
            header,
            package_count
        })
    }
}

/// A collection of resource data types within a package.  Followed by
/// one or more ResTable_type and ResTable_typeSpec structures containing the
/// entry values for each resource type.
///
/// TODO: we do not deal with the following structs
#[derive(Debug)]
pub struct ResTablePackage {
    // Package header
    header: ChunkHeader,

    // If this is a base package, its ID.  Package IDs 
    // at 1 (corresponding to the value of the package bits in a
    // resource identifier).  0 means this is not a base package.
    id: u32,

    // Actual name of this package, \0-terminated.
    name: [u16; 128],

    // Offset to a ResStringPool_header defining the resource
    // type symbol table.  If zero, this package is inheriting from
    // another base package (overriding specific values in it).
    type_strings: u32,

    // Last index into typeStrings that is for public use by others.
    last_public_type: u32,

    // Offset to a ResStringPool_header defining the resource key
    // symbol table.  If zero, this package is inheriting from another
    // base package (overriding specific values in it).
    key_strings: u32,

    // Last index into keyStrings that is for public use by others.
    last_public_key: u32,

    // Type ID offset
    type_id_offset: u32,
}

impl ResTablePackage {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {

        // Go back 2 bytes, to account from the block type
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

        // Parse chunk header
        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTablePackageType)?;

        // Get other members
        let id = axml_buff.read_u32::<LittleEndian>()?;

        let mut name: [u16; 128] = [0; 128];
        for item in &mut name {
            *item = axml_buff.read_u16::<LittleEndian>()?;
            if *item == 0x00 {
                break;
            }
        }
        let type_strings = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_type = axml_buff.read_u32::<LittleEndian>()?;
        let key_strings = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_key = axml_buff.read_u32::<LittleEndian>()?;
        let type_id_offset = axml_buff.read_u32::<LittleEndian>()?;

        // Build and return the object
        Ok(ResTablePackage {
            header,
            id,
            name,
            type_strings,
            last_public_type,
            key_strings,
            last_public_key,
            type_id_offset
        })
    }
}
