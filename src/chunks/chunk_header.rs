#![allow(dead_code)]

//! Representation of a chunk header
//!
//! An AXML document is composed of several chunks, and each chunk has a header.
//! The header is rather small and only contain the type of the chunk (identified
//! by the `ChunkType` enum), the header size, and the chunk size.

use std::io::{
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt,
};
use crate::chunks::chunk_types::ChunkType;

/// Header that appears at the beginning of every chunk
#[derive(Debug)]
pub struct ChunkHeader {
    /// Type identifier for this chunk.
    /// The meaning of this value depends on the containing chunk.
    pub chunk_type: ChunkType,

    /// Size of the chunk header in bytes.
    pub header_size: u16,

    /// Total size of this chunk in bytes.
    pub chunk_size: u32,
}

impl ChunkHeader {
    /// Parse bytes from given buffer into a `ChunkHeader`
    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>, expected_type: ChunkType) -> Result<Self, Error> {
        // Minimum size, for a chunk with no data
        let minimum_size = 8;

        // Get chunk type
        let chunk_type = ChunkType::parse_block_type(axml_buff)
                        .expect("Error: cannot parse block type");

        // Check if this is indeed of the expected type
        if chunk_type != expected_type {
            panic!("Error: unexpected XML chunk type");
        }

        // Get chunk header size and total size
        let header_size = axml_buff.read_u16::<LittleEndian>().unwrap();
        let chunk_size = axml_buff.read_u32::<LittleEndian>().unwrap();

        // Exhaustive checks on the announced sizes
        if header_size < minimum_size {
            panic!("Error: parsed header size is smaller than the minimum");
        }

        if chunk_size < minimum_size.into() {
            panic!("Error: parsed total size is smaller than the minimum");
        }

        if chunk_size < header_size.into() {
            panic!("Error: parsed total size is smaller than parsed header size");
        }

        Ok(ChunkHeader {
            chunk_type,
            header_size,
            chunk_size,
        })
    }

    /// Debug function
    pub fn print(&self) {
        println!("----- Chunk header -----");
        println!("Header chunk_type: {:02X}", self.chunk_type);
        println!("Header header_size: {:02X}", self.header_size);
        println!("Chunk size: {:04X}", self.chunk_size);
        println!("----- End chunk header -----");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ChunkType;

    #[test]
    fn test_valid_case() {
        let valid_data = vec![1, 0, 8, 0, 16, 0, 0, 0];
        let mut cursor = Cursor::new(valid_data);

        let expected_type = ChunkType::ResStringPoolType;
        let result = ChunkHeader::from_buff(&mut cursor, expected_type);

        assert!(result.is_ok());
        let chunk_header = result.unwrap();
        assert_eq!(chunk_header.chunk_type, expected_type);
        assert_eq!(chunk_header.header_size, 8);
        assert_eq!(chunk_header.chunk_size, 16);
    }

    #[test]
    #[should_panic(expected = "Error: unexpected XML chunk type")]
    fn test_unexpected_chunk_type() {
        // Prepare a buffer with a chunk type that doesn't match the expected one
        let invalid_data = vec![2, 0, 8, 0, 16, 0, 0, 0];
        let mut cursor = Cursor::new(invalid_data);

        let expected_type = ChunkType::ResStringPoolType;
        let _ = ChunkHeader::from_buff(&mut cursor, expected_type);
    }

    #[test]
    #[should_panic(expected = "Error: parsed header size is smaller than the minimum")]
    fn test_invalid_header_size() {
        // Prepare a buffer with a small header size (less than 8)
        let invalid_data = vec![1, 0, 4, 0, 16, 0, 0, 0];
        let mut cursor = Cursor::new(invalid_data);

        let expected_type = ChunkType::ResStringPoolType;
        let _ = ChunkHeader::from_buff(&mut cursor, expected_type);
    }

    #[test]
    #[should_panic(expected = "Error: parsed total size is smaller than the minimum")]
    fn test_invalid_chunk_size() {
        // Prepare a buffer with an invalid chunk size (less than 8)
        let invalid_data = vec![1, 0, 8, 0, 4, 0, 0, 0];
        let mut cursor = Cursor::new(invalid_data);

        let expected_type = ChunkType::ResStringPoolType;
        let _ = ChunkHeader::from_buff(&mut cursor, expected_type);
    }

    #[test]
    #[should_panic(expected = "Error: parsed total size is smaller than parsed header size")]
    fn test_invalid_chunk_size_smaller_than_header() {
        // Prepare a buffer where chunk size is smaller than header size
        // Note: the header size is constant and is always 8 bytes which is
        // the minimum allowed. Since we first check that the header size
        // and chunk size are now below 8 before comparing the two sizes
        // this should actually never happen. But we make a test anyway.
        let invalid_data = vec![1, 0, 16, 0, 8, 0, 0, 0];
        let mut cursor = Cursor::new(invalid_data);

        let expected_type = ChunkType::ResStringPoolType;
        let _ = ChunkHeader::from_buff(&mut cursor, expected_type);
    }
}
