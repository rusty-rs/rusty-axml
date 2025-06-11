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

            // let char_count: u16; // This will be handled per branch
            let decoded_string: String;

            if is_utf8 {
                // Read UTF-8 character count (spec calls this "UTF-16 length")
                // This is the number of characters, not bytes.
                let _utf8_char_count: u16; // Mark as unused for now, but read it as per spec.
                let mut first_byte_char_count = axml_buff.read_u8()? as u16;
                if (first_byte_char_count & 0x80) != 0 {
                    first_byte_char_count &= 0x7F; // Mask out the high bit
                    _utf8_char_count = (first_byte_char_count << 8) | (axml_buff.read_u8()? as u16);
                } else {
                    _utf8_char_count = first_byte_char_count;
                }

                // Read UTF-8 byte length (spec calls this "UTF-8 length")
                let mut first_byte_byte_len = axml_buff.read_u8()? as u16;
                let byte_len: u16; // Renamed from _encoded_size
                if (first_byte_byte_len & 0x80) != 0 {
                    first_byte_byte_len &= 0x7F; // Mask out the high bit
                    byte_len = (first_byte_byte_len << 8) | (axml_buff.read_u8()? as u16);
                } else {
                    byte_len = first_byte_byte_len;
                }

                // Use byte_len to read the string data
                let mut str_buff = Vec::with_capacity(byte_len as usize);
                let mut chunk = axml_buff.take(byte_len as u64);

                chunk.read_to_end(&mut str_buff)?;
                decoded_string = String::from_utf8(str_buff)?;
                axml_buff.read_u8()?; // Consume UTF-8 null terminator (0x00)
                                      // TODO: According to AOSP comments for resources.arsc, UTF-8 strings might have 2 or 4 byte terminators.
                                      // This needs verification if parsing resources.arsc strings yields errors or incorrect cursor positions.
            } else { // UTF-16
                let char_count = axml_buff.read_u16::<LittleEndian>()? as u16; // UTF-16 length in characters
                let actual_decoded_string: String;
                if char_count > 0 {
                    let mut str_chars = Vec::with_capacity(char_count as usize);
                    for _ in 0..char_count {
                        // It's possible for a read error here if char_count is erroneously large.
                        // Consider adding error handling or further validation if issues arise.
                        str_chars.push(axml_buff.read_u16::<LittleEndian>()?);
                    }
                    actual_decoded_string = std::char::decode_utf16(str_chars.into_iter())
                                         .collect::<Result<String, _>>()?;
                } else {
                    actual_decoded_string = String::new();
                }
                axml_buff.read_u16::<LittleEndian>()?; // Consume UTF-16 null terminator (0x0000)
                decoded_string = actual_decoded_string;
            }

            // All strings, including empty ones, are added to the pool.
            global_strings.push(decoded_string);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResStringPool_ref {
    /// Index into the string pool table (uint32_t-offset from the indices
    /// immediately after ResStringPool_header) at which to find the location
    /// of the string data in the pool.
    pub index: u32,
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

    #[test]
    fn test_long_utf8_string_parsing() {
        let mut buf = Vec::new();
        let long_string = "A".repeat(150);
        let string_char_count = 150u16; // 0x96
        let string_byte_len = 150u16;   // 0x96

        // Chunk header part of ResStringPool_header
        let chunk_type_res_string_pool: u16 = 0x0001;
        // let chunk_header_size_res_string_pool: u16 = 28; // Standard size for ResStringPool_header (ResChunk_header + ResStringPool_header specific fields)

        // String pool specific header fields
        let string_count_val: u32 = 1;
        let style_count_val: u32 = 0;
        let flags_val: u32 = 256; // UTF-8

        // Calculate offsets and sizes
        // Size of ResStringPool_header specific fields (string_count, style_count, flags, strings_start, styles_start)
        let res_string_pool_specific_fields_size: u32 = 5 * 4; // 20 bytes
        let string_offsets_array_size: u32 = string_count_val * 4; // Each offset is u32, 1 string = 4 bytes

        // String entry: char_count_header (2 bytes) + byte_len_header (2 bytes) + string_itself (150 bytes) + null_terminator (1 byte)
        let single_string_entry_size: u32 = 2 + 2 + string_byte_len as u32 + 1; // 155 bytes

        let styles_start_val: u32 = 0; // No styles, so can be 0. This is an offset from chunk header.

        // ResChunk_header.size: size of this chunk following the ResChunk_header (8 bytes).
        // It includes: ResStringPool_header specific fields (20B) + string_offsets_array (4B) + string_data (155B)
        let chunk_data_size_val: u32 = res_string_pool_specific_fields_size + string_offsets_array_size + single_string_entry_size; // 20 + 4 + 155 = 179 bytes


        // ResChunk_header (8 bytes total)
        buf.write_u16::<LittleEndian>(chunk_type_res_string_pool).unwrap(); // ChunkType (2B)
        buf.write_u16::<LittleEndian>(8).unwrap(); // ResChunk_header.headerSize (2B) - size of this ResChunk_header
        buf.write_u32::<LittleEndian>(chunk_data_size_val).unwrap();    // ResChunk_header.size (4B) - size of chunk data following this header

        // ResStringPool_header specific fields (20 bytes total)
        buf.write_u32::<LittleEndian>(string_count_val).unwrap(); // (4B)
        buf.write_u32::<LittleEndian>(style_count_val).unwrap(); // (4B)
        buf.write_u32::<LittleEndian>(flags_val).unwrap();       // (4B)
        // ResStringPool_header.stringsStart: offset from start of ResChunk_header to string data.
        // String data begins after: ResChunk_header (8B) + ResStringPool_header specific fields (20B) + string_offsets_array (4B)
        let strings_start_field_val: u32 = 8 + res_string_pool_specific_fields_size + string_offsets_array_size; // 8 + 20 + 4 = 32
        buf.write_u32::<LittleEndian>(strings_start_field_val).unwrap(); // stringsStart (4B)
        buf.write_u32::<LittleEndian>(styles_start_val).unwrap();     // stylesStart (4B)

        // String offsets array (4 bytes total for 1 string)
        // Each entry is an offset from ResStringPool_header.stringsStart to the actual string entry.
        // Since our string data immediately follows the string_offsets_array, and stringsStart points to the start of string data,
        // the first string entry is at offset 0 from stringsStart.
        buf.write_u32::<LittleEndian>(0).unwrap(); // Offset of the first string (relative to strings_start_field_val)

        // String data
        // Character count (150)
        buf.write_u8(0x80 | ((string_char_count >> 8) & 0x7F) as u8).unwrap();
        buf.write_u8((string_char_count & 0xFF) as u8).unwrap();

        // Byte length (150)
        buf.write_u8(0x80 | ((string_byte_len >> 8) & 0x7F) as u8).unwrap();
        buf.write_u8((string_byte_len & 0xFF) as u8).unwrap();

        buf.write_all(long_string.as_bytes()).unwrap();
        buf.write_u8(0x00).unwrap();                    // Null terminator

        let mut buffer = Cursor::new(buf);

        // The `from_buff` function assumes we have read the chunk type already
        buffer.read_u16::<LittleEndian>().unwrap(); // Consume chunk type

        let mut global_strings = Vec::new();
        let string_pool = StringPool::from_buff(&mut buffer, &mut global_strings).unwrap();

        assert_eq!(string_pool.strings.len(), 1);
        assert_eq!(string_pool.strings[0], long_string);
        assert_eq!(string_pool.strings[0].len(), 150);
        assert_eq!(string_pool.string_count, 1);
        assert!(string_pool.is_utf8);
    }
}
