#![allow(dead_code)]

//! String pool
//!
//! The string pool is the set of strings used in the AXML files. All
//! of these strings can be then referenced by the chunks. This reduces
//! the size of the binary XML as there is no duplication of strings
//! anymore.

use crate::{
    chunks::{
        chunk_header::ChunkHeader,
        chunk_types::ChunkType,
    },
    errors::AxmlError
};

use std::io::{
    Read,
    Cursor,
};

use byteorder::{
    LittleEndian,
    ReadBytesExt
};

/// String pool structure
///
/// The data of the string pool is an array of `u32` that provides the
/// indices in the pool. The pool itself is located at `strings_start`
/// offset. Each item of the pool is composed of:
///      - the string length (16 bits, more details below)
///      - the string (in UTF-16 format)
///      - a terminator (`0x0000`)
///
/// The length is 16 bits long, but the system only uses 15 bits,
/// which means that the maximum length of a string is 32,676
/// characters. If a string has more than 32767 characters, the high
/// bit of the length is set and the 15 remaining bits represent the
/// high word of the total length. In this case, the length will be
/// immediately followed by another 16 bits which represent the low
/// end of the string length. This means the format allows for string
/// lengths up to 2,147,483,648 characters.
///
/// If `style_count` is not zero, then immediately following the array
/// of indices into the string table is another array of indices into
/// a style table starting at `styles_start`. Each entry in the style
/// table is an array of `string_pool_span` structures.
#[derive(Debug)]
pub struct StringPool {
    /// Chunk header
    header: ChunkHeader,

    /// Number of strings in this pool (that is, number of `u32`
    /// indices that follow in the data)
    string_count: u32,

    /// Number of style span arrays in the pool (that is, number
    /// of `u32` indices follow the string indices)
    style_count: u32,

    /// Flags. There are two possible flags:
    ///     - `is_sorted`: if set, the string pool is sorted by
    ///       UTF-16 string values
    ///     - `is_utf8`: if set, the string pool is encoded in
    ///       UTF-8 and not UTF-16
    is_sorted: bool,
    is_utf8: bool,

    /// Offset from the header to the string data
    strings_start: u32,

    /// Offset from the header to the style data
    styles_start: u32,

    /// List of strings offsets
    strings_offsets: Vec<u32>,

    /// List of styles offsets
    styles_offsets: Vec<u32>,

    /// The strings from the pool
    strings: Vec<String>,

    /// The styles from the pool
    styles: Vec<StringPoolSpan>,
}

impl StringPool {
    /// Parse the string pool from the raw data
    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>,
                 global_strings: &mut Vec<String>) -> Result<Self, AxmlError> {

        // Go back 2 bytes, to account from the block type
        let initial_offset = axml_buff.position() - 2;
        axml_buff.set_position(initial_offset);
        let initial_offset = initial_offset as u32;

        // Parse chunk header
        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResStringPoolType)?;

        // Get remaining members
        let string_count = axml_buff.read_u32::<LittleEndian>()?;
        let style_count = axml_buff.read_u32::<LittleEndian>()?;
        let flags = axml_buff.read_u32::<LittleEndian>()?;
        let is_sorted = (flags & (1<<0)) != 0;
        let is_utf8 = (flags & (1<<8)) != 0;
        let strings_start = axml_buff.read_u32::<LittleEndian>()?;
        let styles_start = axml_buff.read_u32::<LittleEndian>()?;

        // Get strings offsets
        let mut strings_offsets = Vec::new();
        for _ in 0..string_count {
            let offset = axml_buff.read_u32::<LittleEndian>()?;
            strings_offsets.push(offset);
        }

        // Get styles offsets
        let mut styles_offsets = Vec::new();
        for _ in 0..style_count {
            let offset = axml_buff.read_u32::<LittleEndian>()?;
            styles_offsets.push(offset);
        }

        // Strings
        for offset in strings_offsets.iter() {
            // let current_start = (strings_start + offset + 8) as u64;
            let current_start = (initial_offset + strings_start + offset) as u64;
            axml_buff.set_position(current_start);

            let str_size;
            let decoded_string;

            if is_utf8 {
                // NOTE for resources.arsc files
                //
                // Each String entry contains Length header (2 bytes to 4 bytes) + Actual String + [0x00]
                // Length header sometime contain duplicate values e.g. 20 20
                // Actual string sometime contains 00, which need to be ignored
                // Ending zero might be  2 byte or 4 byte
                //
                // TODO: Consider both Length bytes and String length > 32767 characters
                //
                // Actually, there are two length if the file is in UTF-8: the encoded and decoded lengths
                //

                let _encoded_size = axml_buff.read_u8()? as u32;
                str_size = axml_buff.read_u8()? as u32;
                let mut str_buff = Vec::with_capacity(str_size as usize);
                let mut chunk = axml_buff.take(str_size.into());

                chunk.read_to_end(&mut str_buff)?;
                // decoded_string = String::from_utf8(str_buff)?;
                decoded_string = String::from_utf8(str_buff)?;
            } else {
                str_size = axml_buff.read_u16::<LittleEndian>()? as u32;
                // TODO: can we get rid of this unwrap here?
                let iter = (0..str_size as usize)
                    .map(|_| axml_buff.read_u16::<LittleEndian>().unwrap());
                decoded_string = std::char::decode_utf16(iter).collect::<Result<String, _>>()?;
            }

            if str_size > 0 {
                global_strings.push(decoded_string);
            }
        }

        let mut styles = Vec::new();
        for offset in styles_offsets.iter() {
            let current_start = (initial_offset + strings_start + offset) as u64;
            axml_buff.set_position(current_start);

            let string_pool_ref = StringPoolRef {
                index: axml_buff.read_u32::<LittleEndian>()?
            };
            let first_char = axml_buff.read_u32::<LittleEndian>()?;
            let last_char = axml_buff.read_u32::<LittleEndian>()?;

            styles.push(StringPoolSpan {
                name: string_pool_ref,
                first_char,
                last_char
            });
        }

        Ok(StringPool {
            header,
            string_count,
            style_count,
            is_sorted,
            is_utf8,
            strings_start,
            styles_start,
            strings_offsets,
            styles_offsets,
            strings: global_strings.to_vec(),
            styles
        })
    }
}

/// Reference to a string in a string pool.
#[derive(Debug)]
struct StringPoolRef {
    /// Index into the string pool table (uint32_t-offset from the indices
    /// immediately after ResStringPool_header) at which to find the location
    /// of the string data in the pool.
    index: u32,
}

/// String pool span
///
/// This structure defines a span of style information associated
/// with a string in the pool.
#[derive(Debug)]
struct StringPoolSpan {
    /// Name of the span
    ///
    /// This is the name of the XML tag that defined it.
    /// There is a special value END (0xFFFFFFFF) that indicates the
    /// end of an array of spans.
    name: StringPoolRef,

    /// The first of the characters in the string that this span applies to
    first_char: u32,

    /// The last of the characters in the string that this span applies to
    last_char: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ Cursor, Write };
    use byteorder::{LittleEndian, WriteBytesExt};

    // Helper function to create a simple buffer for testing
    fn create_test_buffer() -> Cursor<Vec<u8>> {
        let mut buf = Vec::new();

        // Chunk header
        buf.write_u16::<LittleEndian>(0x0001).unwrap(); // ChunkType::ResStringPoolType
        buf.write_u16::<LittleEndian>(8).unwrap();      // Chunk header size
        buf.write_u32::<LittleEndian>(128).unwrap();    // Chunk data size

        // String pool header
        buf.write_u32::<LittleEndian>(2).unwrap();      // string_count
        buf.write_u32::<LittleEndian>(0).unwrap();      // style_count
        buf.write_u32::<LittleEndian>(1).unwrap();      // flags (is_sorted, not is_utf8)
        buf.write_u32::<LittleEndian>(36).unwrap();     // strings_start
        buf.write_u32::<LittleEndian>(20).unwrap();     // styles_start
        buf.write_u32::<LittleEndian>(0).unwrap();      // first string offset
        buf.write_u32::<LittleEndian>(14).unwrap();     // second string offset

        // Add mock string offsets and string data
        buf.write_u16::<LittleEndian>(5).unwrap(); // Length of first string (UTF-16)
        buf.write_u16::<LittleEndian>(0x0048).unwrap(); // 'H'
        buf.write_u16::<LittleEndian>(0x0065).unwrap(); // 'e'
        buf.write_u16::<LittleEndian>(0x006C).unwrap(); // 'l'
        buf.write_u16::<LittleEndian>(0x006C).unwrap(); // 'l'
        buf.write_u16::<LittleEndian>(0x006F).unwrap(); // 'o'
        buf.write_u16::<LittleEndian>(0x0000).unwrap(); // Null terminator

        buf.write_u16::<LittleEndian>(5).unwrap(); // Length of second string (UTF-16)
        buf.write_u16::<LittleEndian>(0x0057).unwrap(); // 'W'
        buf.write_u16::<LittleEndian>(0x006F).unwrap(); // 'o'
        buf.write_u16::<LittleEndian>(0x0072).unwrap(); // 'r'
        buf.write_u16::<LittleEndian>(0x006C).unwrap(); // 'l'
        buf.write_u16::<LittleEndian>(0x0064).unwrap(); // 'd'
        buf.write_u16::<LittleEndian>(0x0000).unwrap(); // Null terminator

        Cursor::new(buf)
    }

    #[test]
    fn test_string_pool_parse_utf16() {
        // Create a test buffer
        let mut buffer = create_test_buffer();

        // The `from_buff` function assumes we have read the chunk type already
        buffer.read_u16::<LittleEndian>().unwrap();

        let mut global_strings = Vec::new();

        // Parse string pool from buffer
        let string_pool = StringPool::from_buff(&mut buffer, &mut global_strings).unwrap();

        // Validate that the string pool is parsed correctly
        assert_eq!(string_pool.strings.len(), 2);
        assert_eq!(string_pool.strings[0], "Hello");
        assert_eq!(string_pool.strings[1], "World");
    }

    #[test]
    fn test_string_pool_flags() {
        let mut buffer = create_test_buffer();

        // The `from_buff` function assumes we have read the chunk type already
        buffer.read_u16::<LittleEndian>().unwrap();

        let mut global_strings = Vec::new();

        // Parse string pool from buffer
        let string_pool = StringPool::from_buff(&mut buffer, &mut global_strings).unwrap();

        // Validate the flags
        assert!(string_pool.is_sorted);
        assert!(!string_pool.is_utf8);
    }

    #[test]
    fn test_empty_pool() {
        // Test case with no strings in the pool
        let mut buf = Vec::new();

        buf.write_u16::<LittleEndian>(0x0001).unwrap(); // ChunkType::ResStringPoolType
        buf.write_u16::<LittleEndian>(8).unwrap();      // Chunk header size
        buf.write_u32::<LittleEndian>(128).unwrap();    // Chunk data size

        buf.write_u32::<LittleEndian>(0).unwrap(); // string_count = 0
        buf.write_u32::<LittleEndian>(0).unwrap(); // style_count = 0
        buf.write_u32::<LittleEndian>(0).unwrap(); // flags
        buf.write_u32::<LittleEndian>(32).unwrap(); // strings_start
        buf.write_u32::<LittleEndian>(20).unwrap(); // styles_start

        let mut buffer = Cursor::new(buf);

        // The `from_buff` function assumes we have read the chunk type already
        buffer.read_u16::<LittleEndian>().unwrap();

        let mut global_strings = Vec::new();

        let string_pool = StringPool::from_buff(&mut buffer, &mut global_strings).unwrap();

        // Check that the string pool is correctly parsed and contains no strings
        assert_eq!(string_pool.strings.len(), 0);
    }

    #[test]
    fn test_utf8_string_parsing() {
        // UTF-8 encoded string with length 5 (using mock data for simplicity)
        let mut buf = Vec::new();

        buf.write_u16::<LittleEndian>(0x0001).unwrap(); // ChunkType::ResStringPoolType
        buf.write_u16::<LittleEndian>(8).unwrap();      // Chunk header size
        buf.write_u32::<LittleEndian>(128).unwrap();    // Chunk data size

        buf.write_u32::<LittleEndian>(1).unwrap();      // string_count = 1
        buf.write_u32::<LittleEndian>(0).unwrap();      // style_count = 0
        buf.write_u32::<LittleEndian>(256).unwrap();    // flags (not sorted, utf8)
        buf.write_u32::<LittleEndian>(32).unwrap();     // strings_start
        buf.write_u32::<LittleEndian>(20).unwrap();     // styles_start
        buf.write_u32::<LittleEndian>(0).unwrap();      // Offset of the string

        buf.write_u8(0x05).unwrap();                    // UTF-8 string length
        buf.write_u8(0x05).unwrap();                    // UTF-8 string decoded length
        buf.write_all(b"Hello").unwrap();          // UTF-8 string data
        buf.write_u8(0x00).unwrap();                    // Null terminator

        let mut buffer = Cursor::new(buf);

        // The `from_buff` function assumes we have read the chunk type already
        buffer.read_u16::<LittleEndian>().unwrap();

        let mut global_strings = Vec::new();

        let string_pool = StringPool::from_buff(&mut buffer, &mut global_strings).unwrap();

        // Validate that the string pool has correctly decoded the UTF-8 string
        assert_eq!(string_pool.strings.len(), 1);
        assert_eq!(string_pool.strings[0], "Hello");
    }
}
