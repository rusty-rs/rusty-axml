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
    pub header: ChunkHeader, // Made public for tests if needed

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
        // The cursor is expected to be at the start of the ResTable chunk type (u16).
        // So, the start of this ResTable chunk is axml_buff.position() - 2 (if type already read by a peeker).
        // However, our ResTable::parse is usually the entry point, so cursor is at start of file or ResTableChunk.
        // For robustness, let's assume axml_buff.position() is the true start of the ResTable chunk header.
        // If a higher-level parser has already read the chunk type, it should adjust the cursor before calling this.
        // For now, assume the ResTable CHUNK_TYPE itself has NOT been read yet.
        // So, ChunkHeader::from_buff will read it.

        let res_table_chunk_start_pos = axml_buff.position();

        // Parse chunk header
        let res_table_header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableType)?;

        // Get package count
        let package_count_val = axml_buff.read_u32::<LittleEndian>()?;

        // The first chunk after ResTable header's package_count is usually a global string pool.
        let mut parsed_global_string_pool: Option<Vec<String>> = None;
        let mut parsed_packages = Vec::with_capacity(package_count_val as usize);

        // Calculate the end of the ResTable chunk to avoid reading beyond it.
        let res_table_content_end_offset = res_table_chunk_start_pos + res_table_header.size as u64;


        // First, try to parse the global string pool.
        // The cursor is currently after package_count_val.
        // Check if there's enough data for a chunk header before trying to parse.
        if axml_buff.position() < res_table_content_end_offset && (res_table_content_end_offset - axml_buff.position()) >= 8 { // 8 is min chunk header size
            // Peek the next chunk type. Store current position to restore if not a string pool.
            let pos_before_peek = axml_buff.position();
            let next_chunk_type = ChunkType::parse_block_type(axml_buff)?; // Consumes 2 bytes for type

            if next_chunk_type == ChunkType::ResStringPoolType {
                // It is a string pool. StringPool::from_buff expects cursor at start of its header_size field.
                // ChunkHeader::from_buff (called by StringPool::from_buff indirectly or directly)
                // expects cursor at start of type field. parse_block_type already read it.
                // So, StringPool::from_buff needs to be aware of this.
                // Assuming StringPool::from_buff handles this by taking cursor after type.
                let mut strings = Vec::new();
                // StringPool::from_buff internally calls ChunkHeader::from_buff, which will use the current cursor pos - 2.
                // This is correct as parse_block_type advanced it by 2.
                StringPool::from_buff(axml_buff, &mut strings)?;
                parsed_global_string_pool = Some(strings);
            } else {
                // It's not a string pool, restore cursor to before peeking type.
                axml_buff.set_position(pos_before_peek);
            }
        }

        // Loop to parse ResTablePackage chunks
        // We loop up to package_count_val times, but also check against res_table_content_end_offset
        // It's also possible that actual packages are fewer than package_count_val if the file is malformed.
        while parsed_packages.len() < package_count_val as usize &&
              axml_buff.position() < res_table_content_end_offset &&
              (res_table_content_end_offset - axml_buff.position()) >= 8 // Ensure enough for a header
        {
            let pos_before_package_parse = axml_buff.position();
            let block_type = ChunkType::parse_block_type(axml_buff)?;
            match block_type {
                ChunkType::ResTablePackageType => {
                    // Similar to StringPool, ResTablePackage::parse needs to handle that type is already read.
                    // Its from_buff_direct or similar logic should be used if it exists,
                    // or it should internally call ChunkHeader::from_buff which does pos-2.
                    let package = ResTablePackage::parse(axml_buff)?;
                    parsed_packages.push(package);
                },
                _ => {
                    // Unexpected chunk type. This might indicate a malformed ARSC or that we've read past packages.
                    // Restore cursor and break, as we can't proceed with package parsing.
                    axml_buff.set_position(pos_before_package_parse);
                    eprintln!("Warning: Unexpected chunk type {:04X} at offset {} while expecting ResTablePackageType. Parsed {}/{} packages.",
                              block_type as u16, pos_before_package_parse, parsed_packages.len(), package_count_val);
                    break;
                }
            }
        }

        // Ensure cursor is at the end of the ResTable chunk
        axml_buff.set_position(res_table_content_end_offset);

        Ok(Self {
            header: res_table_header,
            package_count: package_count_val, // This is the declared count
            global_string_pool: parsed_global_string_pool,
            packages: parsed_packages, // This is the actually parsed count
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::errors::AxmlError;
    use byteorder::{LittleEndian, WriteBytesExt};

    #[test]
    fn test_parse_config_minimal_size() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        // Write ResTable_config.size = 4. This is the smallest possible valid size,
        // indicating that only the size field itself is present, and no other data follows for this struct.
        data.write_u32::<LittleEndian>(4).unwrap();
        // mcc, mnc, etc., are not written to data, so parse should default them.

        let mut cursor = Cursor::new(data);
        let config = ResTable_config::parse(&mut cursor)?;

        assert_eq!(config.size, 4);
        // For size = 4, parse() reads 'size' and then tries to read mcc, mnc, etc.
        // Since there's no more data, read_u16/read_u8 will return EOF errors.
        // The current ResTable_config::parse reads these unconditionally after 'size'.
        // This test will fail if not handled.
        // The parse logic needs to be robust enough not to read past the provided 'size'.

        // Based on current ResTable_config::parse logic:
        // It reads `size`. Then it reads `mcc`, `mnc`, `language`, `country`, `orientation`,
        // `touchscreen`, `density`, `keyboard`, `navigation`, `input_flags`, `input_pad0`,
        // `screen_width`, `screen_height`, `sdk_version`, `minor_version` *unconditionally*.
        // This totals 4 (size) + 2+2+2+2+1+1+2+1+1+1+1+2+2+2+2 = 27 bytes.
        // If size is 4, it will attempt to read 23 more bytes than available.

        // To make this test pass with current ResTable_config::parse, we'd need to provide
        // at least 28 bytes of data for the initial fixed-size fields it tries to read,
        // or the parse function needs to be more careful based on the initial `size` field.

        // Let's assume the intention of the original ResTable_config::parse was that if `size`
        // is very small (e.g. less than the size of even the first few fields), those fields
        // are effectively zero/default. The "if size >= X" checks later handle optional fields.

        // If ResTable_config::parse were to be fixed to handle size=4 correctly by not reading further,
        // then the following assertions would be for the default values.
        // For now, this test highlights a potential issue in ResTable_config::parse for small sizes.
        // To proceed with the test as written (expecting defaults for size=4), ResTable_config::parse
        // would need modification.

        // Given the current implementation of ResTable_config::parse,
        // a "minimal" test where it *successfully* parses without error would involve providing
        // at least the number of bytes it unconditionally tries to read after the `size` field.
        // The smallest `size` it can parse without error for the fixed part is 28.
        // If size is 4, it will error out. The test below (`test_parse_config_actually_minimal_successful`)
        // explores what would be a minimal *successful* parse.

        // This test, as requested for size=4, will fail due to read errors.
        // For it to pass by returning default values, ResTable_config::parse must be changed.
        // If the goal is to test that it *errors out* for size=4, the test should assert an Err result.
        // Let's assume the goal is that it *should* parse and return defaults.
        // This means the current ResTable_config::parse is not robust for size < header/fixed fields.

        // For the purpose of this exercise, I will write the assertions as if parse() was robust
        // and returned defaults when size doesn't allow reading fields.
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
        data.write_u32::<LittleEndian>(36).unwrap(); // size = 36 (payload is 32 bytes)
        data.write_u16::<LittleEndian>(120).unwrap(); // mcc (offset 0-1 in payload)
        data.write_u16::<LittleEndian>(240).unwrap(); // mnc (offset 2-3 in payload)
        data.write_all(b"en").unwrap(); // language (offset 4-5 in payload)
        data.write_all(b"US").unwrap(); // country (offset 6-7 in payload)
        data.write_u8(1).unwrap();      // orientation (offset 8 in payload)
        data.write_u8(2).unwrap();      // touchscreen (offset 9 in payload)
        data.write_u16::<LittleEndian>(480).unwrap(); // density (offset 10-11 in payload)
        data.write_u8(1).unwrap();      // keyboard (offset 12 in payload)
        data.write_u8(2).unwrap();      // navigation (offset 13 in payload)
        data.write_u8(1).unwrap();      // input_flags (offset 14 in payload)
        data.write_u8(0).unwrap();      // input_pad0 (offset 15 in payload)
        data.write_u16::<LittleEndian>(1920).unwrap(); // screen_width (offset 16-17 in payload)
        data.write_u16::<LittleEndian>(1080).unwrap(); // screen_height (offset 18-19 in payload)
        data.write_u16::<LittleEndian>(30).unwrap();   // sdk_version (offset 20-21 in payload)
        data.write_u16::<LittleEndian>(0).unwrap();    // minor_version (offset 22-23 in payload)

        // Fields for size >= 28 (payload offset 24)
        data.write_u8(0x02 | 0x20).unwrap(); // screen_layout (offset 24 in payload)
        data.write_u8(0x01 | 0x00).unwrap(); // ui_mode (offset 25 in payload)
        // Fields for size >= 32 (payload offset 26)
        data.write_u16::<LittleEndian>(360).unwrap(); // smallest_screen_width_dp (offset 26-27 in payload)

        // Fields for size >= 36 (payload offset 28)
        // screen_width_dp (offset 28-29)
        // screen_height_dp (offset 30-31)
        // Total payload written so far: 2 + 2 + 2 + 2 + 1 + 1 + 2 + 1 + 1 + 1 + 1 + 2 + 2 + 2 + 2 + 1 + 1 + 2 = 28 bytes
        // For size = 36, the payload is 32 bytes.
        // We have written 28 bytes of payload. We need to write 4 more bytes for screen_width_dp and screen_height_dp.
        data.write_u16::<LittleEndian>(380).unwrap(); // screen_width_dp (offset 28-29 in payload)
        data.write_u16::<LittleEndian>(420).unwrap(); // screen_height_dp (offset 30-31 in payload)


        // data.len() should be 36 (4 for size + 32 for payload)
        assert_eq!(data.len(), 4 + 32, "Data written does not match expected payload size for config.size=36");

        let mut cursor = Cursor::new(data);
        let config = ResTable_config::parse(&mut cursor)?;

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

        // Assert that fields beyond the 36-byte read are default
        assert_eq!(config.locale_script, [0,0,0,0]);
        assert_eq!(config.locale_variant, [0,0,0,0,0,0,0,0]);
        assert_eq!(config.screen_layout2, 0);
        assert_eq!(config.screen_pad2, 0);
        assert_eq!(config.screen_pad3, 0);

        // Check that the cursor is at the end of what was declared by config.size
        assert_eq!(cursor.position(), config.size as u64, "Cursor not positioned at the end of config.size");

        Ok(())
    }

    #[test]
    fn test_parse_config_full_size_52() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        let config_size = 52u32; // All fields currently in ResTable_config
        data.write_u32::<LittleEndian>(config_size).unwrap(); // size

        data.write_u16::<LittleEndian>(1).unwrap(); // mcc
        data.write_u16::<LittleEndian>(2).unwrap(); // mnc
        data.write_all(b"ja").unwrap(); // language
        data.write_all(b"JP").unwrap(); // country
        data.write_u8(3).unwrap();      // orientation
        data.write_u8(4).unwrap();      // touchscreen
        data.write_u16::<LittleEndian>(320).unwrap(); // density
        data.write_u8(5).unwrap();      // keyboard
        data.write_u8(6).unwrap();      // navigation
        data.write_u8(7).unwrap();      // input_flags
        data.write_u8(8).unwrap();      // input_pad0
        data.write_u16::<LittleEndian>(800).unwrap();  // screen_width
        data.write_u16::<LittleEndian>(600).unwrap();  // screen_height
        data.write_u16::<LittleEndian>(28).unwrap();   // sdk_version
        data.write_u16::<LittleEndian>(0).unwrap();    // minor_version (must be 0)
        data.write_u8(9).unwrap();      // screen_layout
        data.write_u8(10).unwrap();     // ui_mode
        data.write_u16::<LittleEndian>(320).unwrap(); // smallest_screen_width_dp
        data.write_u16::<LittleEndian>(600).unwrap(); // screen_width_dp
        data.write_u16::<LittleEndian>(800).unwrap(); // screen_height_dp
        data.write_all(b"Latn").unwrap(); // locale_script
        data.write_all(&[b'V', b'A', b'R', b'I', b'A', b'N', b'T', 0]).unwrap(); // locale_variant
        data.write_u8(11).unwrap();     // screen_layout2
        data.write_u8(12).unwrap();     // screen_pad2
        data.write_u16::<LittleEndian>(13).unwrap(); // screen_pad3

        assert_eq!(data.len() - 4, config_size as usize - 4, "Payload size mismatch");

        let initial_cursor_pos = 0; // Assuming cursor starts at the beginning of this specific data block
        let mut cursor = Cursor::new(data);
        let config = ResTable_config::parse(&mut cursor)?;

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
        let config_size = 28u32; // Up to minor_version
        data.write_u32::<LittleEndian>(config_size).unwrap(); // size

        data.write_u16::<LittleEndian>(10).unwrap(); // mcc
        data.write_u16::<LittleEndian>(20).unwrap(); // mnc
        data.write_all(b"fr").unwrap(); // language
        data.write_all(b"CA").unwrap(); // country
        data.write_u8(1).unwrap();      // orientation
        data.write_u8(1).unwrap();      // touchscreen
        data.write_u16::<LittleEndian>(240).unwrap(); // density
        data.write_u8(2).unwrap();      // keyboard
        data.write_u8(1).unwrap();      // navigation
        data.write_u8(0).unwrap();      // input_flags
        data.write_u8(0).unwrap();      // input_pad0
        data.write_u16::<LittleEndian>(1280).unwrap(); // screen_width
        data.write_u16::<LittleEndian>(720).unwrap();  // screen_height
        data.write_u16::<LittleEndian>(25).unwrap();   // sdk_version
        data.write_u16::<LittleEndian>(0).unwrap();    // minor_version
        // Total payload written: 24 bytes. config_size is 28 (4 for size + 24 for payload)
        assert_eq!(data.len() - 4, config_size as usize - 4, "Payload size mismatch");


        let initial_cursor_pos = 0;
        let mut cursor = Cursor::new(data);
        let config = ResTable_config::parse(&mut cursor)?;

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

        // Assert that fields beyond this size are default
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
        data.write_u32::<LittleEndian>(52).unwrap(); // Declare size = 52
        data.extend_from_slice(&[0u8; 30]); // But only provide 30 bytes of payload (total 34 bytes in buffer)
                                            // This means it expects 52-4 = 48 bytes of payload.

        let mut cursor = Cursor::new(data);
        let result = ResTable_config::parse(&mut cursor);

        // The parse logic will try to read up to 52 bytes from initial_cursor_pos.
        // If initial_cursor_pos is 0, it will try to read up to offset 52.
        // The buffer only has 4 (size) + 30 (payload) = 34 bytes.
        // The final axml_buff.set_position(initial_cursor_pos + config.size as u64) will cause an error
        // if config.size is larger than the buffer, as set_position might not error but subsequent reads would.
        // However, an earlier read within the conditional blocks might fail if a field is partially present.
        // Given that `axml_buff.read_exact` or `read_u16` would fail if not enough bytes are available
        // for that specific read operation when `config.size` allows it.

        // Let's trace:
        // config.size = 52.
        // It will try to read all fields.
        // locale_script (offset 32, size 4): needs up to 4+32+4 = 40. Buffer has 34. This read will fail.
        assert!(matches!(result, Err(AxmlError::IoError(_))), "Expected IoError due to insufficient data for declared size.");
    }

    #[test]
    fn test_parse_config_zero_size() -> Result<(), AxmlError> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(0).unwrap(); // size = 0

        let initial_cursor_pos = 0;
        let mut cursor = Cursor::new(data);
        let config = ResTable_config::parse(&mut cursor)?;

        assert_eq!(config.size, 0);
        let default_config = ResTable_config::default();
        assert_eq!(config.mcc, default_config.mcc);
        assert_eq!(config.mnc, default_config.mnc);
        assert_eq!(config.language, default_config.language);
        assert_eq!(config.country, default_config.country);
        assert_eq!(config.orientation, default_config.orientation);
        assert_eq!(config.touchscreen, default_config.touchscreen);
        assert_eq!(config.density, default_config.density);
        // ... (assert all other fields are default)
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

        assert_eq!(cursor.position(), initial_cursor_pos + config.size as u64, "Cursor position should be initial_pos + 0 for size=0"); // initial_pos + 0
        Ok(())
    }

    #[test]
    fn test_parse_type_spec_basic() -> Result<(), AxmlError> {
        let mut test_data = Vec::new(); // Renamed to avoid conflict with test name in prompt
        // ChunkHeader: type=ResTableTypeSpecType, headerSize=8 (standard ResChunk_header size for this)
        // ResTable_typeSpec body fields: id (1), res0 (1), res1 (2), entry_count (4) = 8 bytes
        // For entryCount = 2, entry_flags will be 2 * 4 = 8 bytes.
        // Total chunk size = 8 (header) + 8 (body) + 8 (entry_flags) = 24 bytes.

        // Write ResChunk_header part
        data.write_u16::<LittleEndian>(ChunkType::ResTableTypeSpecType as u16).unwrap(); // type
        data.write_u16::<LittleEndian>(16).unwrap();   // headerSize for ResTable_typeSpec (8 for ResChunk_header + 8 for specific fields)
                                                      // ResourceTypes.h: struct ResTable_typeSpec : public ResChunk_header { uint8_t id; ... uint32_t entryCount; }
                                                      // The headerSize should be for ResTable_typeSpec itself, which is 16 bytes.
                                                      // The ResChunk_header.headerSize inside it would be 8.
                                                      // The prompt has headerSize=8, which is for ResChunk_header, not ResTable_typeSpec.
                                                      // Let's assume the ResTable_typeSpec::parse expects to read its *own* specific header fields
                                                      // *after* a generic ChunkHeader::from_buff has read the ResChunk_header.
                                                      // If ResTable_typeSpec::parse calls ChunkHeader::from_buff, then it will read 8 bytes for header.
                                                      // The `ResTable_typeSpec::parse` function as implemented calls `ChunkHeader::from_buff`
                                                      // which reads the 8-byte `ResChunk_header`.
                                                      // So, the provided headerSize in data should be 8 for that part.
        data.write_u16::<LittleEndian>(8).unwrap();    // ChunkHeader.headerSize (for ResChunk_header part)
        data.write_u32::<LittleEndian>(16 + 8).unwrap();   // Total chunk size: 16 (ResTable_typeSpec specific header) + 8 (for 2 flags) = 24
                                                      // ResTable_typeSpec specific header is id(1)+res0(1)+res1(2)+entry_count(4) = 8 bytes.
                                                      // So, if ChunkHeader.headerSize is 8, then size must be 8 + 8 (spec_body) + 8 (flags) = 24.
                                                      // If ResTable_typeSpec struct includes the ResChunk_header, its header_size is 16.

        // The current ResTable_typeSpec::parse calls ChunkHeader::from_buff, which reads the 8-byte generic header.
        // Then it reads id, res0, res1, entry_count. These are part of ResTable_typeSpec's own "header" fields,
        // not part of the entry_flags data that follows.
        // The ChunkHeader.size should be the total size of this chunk from this point.
        // If ChunkHeader.headerSize is 8 (for ResChunk_header):
        //  - size (u32) = 24 (total)
        // After ResChunk_header (8 bytes), 16 bytes remain.
        // These 16 bytes are: id(1), res0(1), res1(2), entry_count(4)  => 8 bytes
        // entry_flags (entry_count * 4) => 2 * 4 = 8 bytes
        // This matches.

        // ResTable_typeSpec body
        data.write_u8(1).unwrap();              // id
        data.write_u8(0).unwrap();              // res0
        data.write_u16::<LittleEndian>(0).unwrap(); // res1
        data.write_u32::<LittleEndian>(2).unwrap(); // entryCount = 2

        // Entry flags
        data.write_u32::<LittleEndian>(0x00000001).unwrap();
        data.write_u32::<LittleEndian>(0x40000000).unwrap();

        // The ResTable_typeSpec::parse function does axml_buff.set_position(initial_offset - 2)
        // assuming it was called after a parse_block_type.
        // For a standalone unit test, we should position the cursor at the start of the chunk type.
        let mut cursor = Cursor::new(data);

        // To simulate how it's called via ResTablePackage::parse:
        // 1. ChunkType::parse_block_type is called (consumes 2 bytes: 0x0202)
        // 2. ResTable_typeSpec::parse is called. It does set_position(current_pos - 2)
        // So, for the test, we should provide the type to parse_block_type, or directly call parse
        // with the cursor already past the type, and then the internal rewind will work.
        // Or, more simply, ensure the data starts with the type, and parse() handles it.
        // The current ResTable_typeSpec::parse rewinds, then calls ChunkHeader::from_buff.
        // ChunkHeader::from_buff expects to read the type and headerSize itself.

        // Let's provide the data from the absolute start of the chunk.
        // `ResTable_typeSpec::parse` will internally do `axml_buff.set_position(initial_offset - 2);`
        // This means if we pass a cursor that's conceptually *after* `parse_block_type` (i.e. type already read),
        // it will rewind to read the type again for `ChunkHeader::from_buff`.
        // So, the data should start with the type.
        // `ChunkHeader::from_buff` will read type (2), headerSize (2), size (4).
        // Then `ResTable_typeSpec::parse` reads id, res0, res1, entry_count.

        // The `ResTable_typeSpec::parse` method is called after `ChunkType::parse_block_type`.
        // `parse_block_type` advances the cursor by 2 bytes.
        // `ResTable_typeSpec::parse` then does `axml_buff.set_position(initial_offset - 2);`
        // So, effectively, `ChunkHeader::from_buff` is called with the cursor at the beginning of the chunk.

        // Data for test:
        // Type (u16), HeaderSize (u16), ChunkSize (u32) -> For ResChunk_header
        // id (u8), res0 (u8), res1 (u16), entry_count (u32) -> For ResTable_typeSpec specific fields
        // entry_flags (Vec<u32>)

        // ResChunk_header part
        // Type: 0x0202 (ResTableTypeSpecType)
        // HeaderSize: 16 (size of ResTable_typeSpec specific header fields, NOT ResChunk_header's size) -- This is a common confusion point.
        // AOSP's ResChunk_header.headerSize for a ResTable_typeSpec chunk is indeed 16.
        // The `ChunkHeader::from_buff` will read the first 8 bytes.
        // The `ResTable_typeSpec::parse` will then read its specific fields.
        // The `header.header_size` field in the *data* should be for the specific chunk type if it's larger than base ResChunk_header.
        // But our `ChunkHeader::from_buff` reads the generic 8-byte header.
        // The `ResTable_typeSpec` struct itself contains a `ChunkHeader` field.
        // Let's assume the headerSize in the file for this chunk is for ResTable_typeSpec (16 bytes)
        // and size is total size (16 for header + 8 for 2 flags = 24).
        // When ChunkHeader::from_buff is called, it will parse:
        // type=0x0202, headerSize=16, size=24. This implies the ResTable_typeSpec specific fields start *after* these 16 bytes.
        // This is NOT how ResTable_typeSpec is structured. It *embeds* ResChunk_header.
        // ResTable_typeSpec { header: ResChunk_header, id, res0, res1, entry_count ... }
        // So, ResChunk_header.headerSize must be >= 16.
        // And ResChunk_header.size must be >= ResChunk_header.headerSize.
        // If ResChunk_header.headerSize = 16:
        //   - Bytes 0-1: type (0x0202)
        //   - Bytes 2-3: headerSize (16)
        //   - Bytes 4-7: size (e.g., 16 + 2*4 = 24 for two flags)
        //   - Bytes 8-8: id (1)
        //   - Bytes 9-9: res0 (0)
        //   - Bytes 10-11: res1 (0)
        //   - Bytes 12-15: entry_count (2)
        //   - Bytes 16-19: flag1
        //   - Bytes 20-23: flag2
        // This aligns with how ResTable_typeSpec::parse reads fields after calling ChunkHeader::from_buff.

        // Corrected data setup:
        // let mut test_data = Vec::new(); // Already declared and renamed
        test_data.write_u16::<LittleEndian>(ChunkType::ResTableTypeSpecType as u16).unwrap(); // type
        test_data.write_u16::<LittleEndian>(16).unwrap(); // headerSize for ResTable_typeSpec
        test_data.write_u32::<LittleEndian>(16 + 2 * 4).unwrap(); // size = 16 (header) + 8 (2 flags) = 24

        test_data.write_u8(1).unwrap();              // id (this is part of the 16 bytes of header)
        test_data.write_u8(0).unwrap();              // res0
        test_data.write_u16::<LittleEndian>(0).unwrap(); // res1
        test_data.write_u32::<LittleEndian>(2).unwrap(); // entryCount = 2

        // Entry flags (these come AFTER the header of size 16)
        test_data.write_u32::<LittleEndian>(0x00000001).unwrap();
        test_data.write_u32::<LittleEndian>(0x40000000).unwrap();

        // Make a new cursor for this corrected data
        let mut cursor_for_spec = Cursor::new(test_data);


        // Simulate the calling context: parse_block_type would have read the type.
        // However, ResTable_typeSpec::parse internally rewinds and re-reads the header.
        // So, we provide the cursor at the start of the chunk.
        let type_spec = ResTable_typeSpec::parse(&mut cursor_for_spec)?;

        assert_eq!(type_spec.header.type_, ChunkType::ResTableTypeSpecType);
        assert_eq!(type_spec.header.header_size, 16); // headerSize specific to ResTable_typeSpec
        assert_eq!(type_spec.header.size, 24);      // Total chunk size

        assert_eq!(type_spec.id, 1);
        assert_eq!(type_spec.res0, 0);
        assert_eq!(type_spec.res1, 0);
        assert_eq!(type_spec.entry_count, 2);
        assert_eq!(type_spec.entry_flags, vec![0x00000001, 0x40000000]);

        // Check cursor position: should be at the end of the chunk
        assert_eq!(cursor_for_spec.position(), 24);

        Ok(())
    }

    #[test]
    fn test_parse_res_table_type_basic() -> Result<(), AxmlError> {
        let mut data = Vec::new();

        // --- ResChunk_header for ResTable_type ---
        // Fixed fields in ResTable_type: id (1), flags (1), reserved (2), entryCount (4), entriesStart (4) = 12 bytes
        // ResChunk_header size (8) + these 12 bytes = 20 bytes for ResTable_type's specific header portion.
        let type_specific_header_size: u16 = 8 + 12;

        let config_data_size: u32 = 36; // Using a known size for ResTable_config payload (excluding its own size field)
                                       // The actual ResTable_config struct on disk will have its own u32 size field.
                                       // So, total bytes for config on disk = 4 + config_data_size_payload if config_data_size_payload means content.
                                       // Let's use the ResTable_config.size field value directly.
        let res_config_struct_size: u32 = 36; // This is the value ResTable_config.size will hold.

        let entry_count_val: u32 = 2;
        let entry_offsets_array_size: u32 = entry_count_val * 4; // Each offset is u32

        // Total chunk size for ResTable_type = type_specific_header_size + res_config_struct_size + entry_offsets_array_size
        let total_chunk_size: u32 = type_specific_header_size as u32 + res_config_struct_size + entry_offsets_array_size;

        data.write_u16::<LittleEndian>(ChunkType::ResTableTypeType as u16).unwrap(); // ChunkType
        data.write_u16::<LittleEndian>(type_specific_header_size).unwrap(); // ResTable_type's headerSize
        data.write_u32::<LittleEndian>(total_chunk_size).unwrap(); // Total chunk size for ResTable_type

        // --- ResTable_type specific fields (12 bytes) ---
        data.write_u8(1).unwrap();              // id (e.g., for "string" type)
        data.write_u8(0).unwrap();              // flags (e.g., 0 for dense)
        data.write_u16::<LittleEndian>(0).unwrap(); // reserved
        data.write_u32::<LittleEndian>(entry_count_val).unwrap(); // entryCount

        // entriesStart is an offset from the beginning of this ResTable_type chunk's own header.
        // It points to where the ResTable_entry data would begin IF it were packed right after this chunk.
        // For this test, we don't have actual entry data, but entriesStart should point
        // beyond this chunk's own data (header + config + offsets_array).
        // Or, more typically, it's an offset within a larger data block where entries are stored.
        // Let's make it point immediately after this ResTable_type chunk for simplicity in this test.
        let entries_start_val_offset: u32 = total_chunk_size;
        data.write_u32::<LittleEndian>(entries_start_val_offset).unwrap();

        // --- ResTable_config data (36 bytes total for the config struct) ---
        data.write_u32::<LittleEndian>(res_config_struct_size).unwrap(); // ResTable_config.size
        data.write_u16::<LittleEndian>(0).unwrap(); // mcc
        data.write_u16::<LittleEndian>(0).unwrap(); // mnc
        data.write_all(b"en").unwrap();      // language
        data.write_all(b"US").unwrap();      // country
        data.write_u8(0).unwrap();           // orientation
        data.write_u8(0).unwrap();           // touchscreen
        data.write_u16::<LittleEndian>(0).unwrap(); // density
        data.write_u8(0).unwrap();           // keyboard
        data.write_u8(0).unwrap();           // navigation
        data.write_u8(0).unwrap();           // inputFlags
        data.write_u8(0).unwrap();           // inputPad0
        data.write_u16::<LittleEndian>(0).unwrap(); // screenWidth
        data.write_u16::<LittleEndian>(0).unwrap(); // screenHeight
        data.write_u16::<LittleEndian>(0).unwrap(); // sdkVersion
        data.write_u16::<LittleEndian>(0).unwrap(); // minorVersion
        // For config_size = 36, payload is 32 bytes.
        // Above fields: 2+2+2+2+1+1+2+1+1+1+1+2+2+2+2 = 24 bytes.
        // Remaining 32 - 24 = 8 bytes for screenLayout, uiMode, smallestScreenWidthDp, screenWidthDp, screenHeightDp
        data.write_u8(0).unwrap();           // screenLayout (payload offset 24)
        data.write_u8(0).unwrap();           // uiMode (payload offset 25)
        data.write_u16::<LittleEndian>(0).unwrap(); // smallestScreenWidthDp (payload offset 26-27)
        data.write_u16::<LittleEndian>(0).unwrap(); // screenWidthDp (payload offset 28-29)
        data.write_u16::<LittleEndian>(0).unwrap(); // screenHeightDp (payload offset 30-31)

        // --- Entry offsets array (entry_count * 4 bytes) ---
        // For this basic test, use NO_ENTRY for all offsets to avoid parsing actual entries,
        // as the entry data is not provided in this test's buffer.
        data.write_u32::<LittleEndian>(0xFFFFFFFF).unwrap(); // Offset for entry 0 (NO_ENTRY)
        data.write_u32::<LittleEndian>(0xFFFFFFFF).unwrap(); // Offset for entry 1 (NO_ENTRY)

        let mut cursor = Cursor::new(data);
        // The ResTable_type::parse method expects the cursor to be positioned *after* the chunk type (u16)
        // because in real scenarios, ChunkType::parse_block_type would have read it.
        // Then, ResTable_type::parse itself will rewind by 2 bytes.
        // So, for this test, we should simulate that by advancing cursor by 2.
        // However, the current ResTable_type::parse starts with:
        // let type_chunk_header_start_pos = axml_buff.position() - 2;
        // axml_buff.set_position(type_chunk_header_start_pos);
        // let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableTypeType)?;
        // This means it correctly handles being called with the cursor at the start of the chunk data (after type).
        // So, we should provide the cursor at the start of the *entire* chunk data written.

        let res_type = ResTable_type::parse(&mut cursor)?;

        assert_eq!(res_type.header.type_, ChunkType::ResTableTypeType);
        assert_eq!(res_type.header.header_size, type_specific_header_size);
        assert_eq!(res_type.header.size, total_chunk_size);
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

        // The .entries field is parsed based on these offsets. Since we haven't provided
        // the actual ResTable_entry data at those offsets, the parse calls for entries
        // would likely fail or return None if the offsets point outside our limited buffer.
        // For this basic test, we primarily care that ResTable_type itself is parsed.
        // The current ResTable_type::parse will attempt to read entries.
        // If entries_start_val_offset + entry_offset[0] is outside the buffer, it will fail.
        // For offset 0x0, it would try to read at total_chunk_size + 0.
        // This test needs to be constructed carefully if we are to test .entries as well.
        // For now, let's assume NO_ENTRY for all or ensure offsets are within a larger mock buffer
        // if we want to test Some(entry) cases.
        // The current test setup for `entries` inside `ResTable_type::parse` will try to read from `type_chunk_header_start_pos + entries_start_val + entry_offset`.
        // If `entries_start_val_offset` is `total_chunk_size`, and `entry_offset[0]` is 0, it reads at `type_chunk_header_start_pos + total_chunk_size`.
        // This is fine, it will try to read past the end of *this* chunk's data.
        // For this test, we expect `res_type.entries` to contain `None` for NO_ENTRY,
        // and potentially errors or `None` for other offsets if data isn't there.
        // The current `ResTable_entry::parse` would error if data is not present.
        // So, for this test to pass reliably without providing entry data, all offsets should be NO_ENTRY
        // or the test should provide a buffer large enough for attempted reads.
        // Let's assume for this *basic* test, we don't delve into successful .entries parsing.
        // The `ResTable_type::parse` sets cursor to end of chunk, so this is fine.
        // The `entries` parsing loop will just populate `None` for `NO_ENTRY` and might error for others if pointing outside.
        // Let's simplify and assume `entries` are not deeply checked here.
        // The logic for `entries` parsing in `ResTable_type::parse` is:
        // `axml_buff.set_position(absolute_entry_location);`
        // `let entry = crate::chunks::res_table_entry::ResTable_entry::parse(axml_buff)?;`
        // This will try to read from the buffer. If `absolute_entry_location` is outside `data.len()`, `set_position` is fine, but `parse` will fail.

        // Given current setup, the test will fail when parsing entries[0] unless entries_start + offset[0] is within buffer
        // and contains valid entry data, OR offset[0] is NO_ENTRY.
        // For this basic test, we'll just check that the offsets were read.
        // A more advanced test would provide a buffer with actual entry data.
        // With NO_ENTRY for all, res_type.entries should be Vec<None>.
        assert_eq!(res_type.entries.len(), entry_count_val as usize);
        assert!(res_type.entries.iter().all(|e| e.is_none()), "All entries should be None for NO_ENTRY offsets");

        assert_eq!(cursor.position(), total_chunk_size as u64, "Cursor not at the end of the ResTable_type chunk");

        Ok(())
    }
}

/// Chunk for a resource type
#[derive(Debug)]
pub struct ResTable_type {
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
    pub config: ResTable_config,
    /// The array of offsets to resource entries.
    pub entry_offsets: Vec<u32>,
    /// Parsed resource entries.
    pub entries: Vec<Option<crate::chunks::res_table_entry::ResTable_entry>>,
}

impl ResTable_type {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        const NO_ENTRY: u32 = 0xFFFFFFFF;

        // The axml_buff is currently positioned right after the chunk type (u16) has been read.
        // So, the start of this ResTable_type chunk is axml_buff.position() - 2.
        let type_chunk_header_start_pos = axml_buff.position() - 2;

        // Parse the main header for ResTable_type
        // The from_buff method expects the cursor to be at the start of the type field,
        // so we need to adjust it back by 2 bytes.
        axml_buff.set_position(type_chunk_header_start_pos);
        let header = ChunkHeader::from_buff(axml_buff, ChunkType::ResTableTypeType)?;

        // Cursor is now after the header. Read the rest of ResTable_type fields.
        let id = axml_buff.read_u8()?;
        let flags_byte = axml_buff.read_u8()?; // Renamed to avoid conflict with flags field in struct
        let reserved = axml_buff.read_u16::<LittleEndian>()?;
        let entry_count_val = axml_buff.read_u32::<LittleEndian>()?; // Renamed
        let entries_start_val = axml_buff.read_u32::<LittleEndian>()?; // Renamed

        let config = ResTable_config::parse(axml_buff)?;

        let mut entry_offsets_vec = Vec::with_capacity(entry_count_val as usize); // Renamed
        for _ in 0..entry_count_val {
            entry_offsets_vec.push(axml_buff.read_u32::<LittleEndian>()?);
        }

        let pos_after_offsets_array = axml_buff.position();
        let mut parsed_entries = Vec::with_capacity(entry_count_val as usize);

        // The entries_start_val is an offset from the beginning of this ResTable_type chunk's header.
        // So, absolute_base_of_entry_data is type_chunk_header_start_pos + entries_start_val.
        let absolute_base_of_entry_data = type_chunk_header_start_pos + entries_start_val as u64;

        for i in 0..(entry_count_val as usize) {
            let entry_offset_within_data_area = entry_offsets_vec[i];
            if entry_offset_within_data_area == NO_ENTRY {
                parsed_entries.push(None);
            } else {
                let absolute_entry_location = absolute_base_of_entry_data + entry_offset_within_data_area as u64;
                axml_buff.set_position(absolute_entry_location);
                let entry = crate::chunks::res_table_entry::ResTable_entry::parse(axml_buff)?;
                parsed_entries.push(Some(entry));
            }
        }

        axml_buff.set_position(pos_after_offsets_array); // Restore cursor to after the offsets array.
                                                         // The next chunk (if any) starts after this ResTable_type chunk.
                                                         // The ResTable_type chunk's full size is header.size.
                                                         // So the cursor should finally be at type_chunk_header_start_pos + header.size.
        axml_buff.set_position(type_chunk_header_start_pos + header.size as u64);


        Ok(ResTable_type {
            header,
            id,
            flags: flags_byte, // Use the read byte here
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
#[derive(Debug)]
pub struct ResTable_typeSpec {
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

impl ResTable_typeSpec {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        // Go back 2 bytes, to account from the block type
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

        Ok(ResTable_typeSpec {
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
///
/// TODO: we do not deal with the following structs
#[derive(Debug)]
pub struct ResTablePackage {
    // Package header
    pub header: ChunkHeader,

    // If this is a base package, its ID.  Package IDs 
    // at 1 (corresponding to the value of the package bits in a
    // resource identifier).  0 means this is not a base package.
    pub id: u32,

    // Actual name of this package, \0-terminated.
    pub name: [u16; 128],

    // Offset to a ResStringPool_header defining the resource
    // type symbol table.  If zero, this package is inheriting from
    // another base package (overriding specific values in it).
    pub type_strings_offset: u32,

    // Last index into typeStrings that is for public use by others.
    pub last_public_type: u32,

    // Offset to a ResStringPool_header defining the resource key
    // symbol table.  If zero, this package is inheriting from another
    // base package (overriding specific values in it).
    pub key_strings_offset: u32,

    // Last index into keyStrings that is for public use by others.
    pub last_public_key: u32,

    // Type ID offset
    pub type_id_offset: u32,

    // Parsed string pool for type names
    pub type_string_pool: Option<Vec<String>>,

    // Parsed string pool for key names
    pub key_string_pool: Option<Vec<String>>,

    // Vector of type specifications
    pub type_specs: Vec<ResTable_typeSpec>,

    // Vector of types
    pub types: Vec<ResTable_type>,
}

impl ResTablePackage {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        // Store the initial position, accounting for the 2 bytes already read for block type
        let package_chunk_start_pos = axml_buff.position() - 2;

        // Parse chunk header
        let package_header = ChunkHeader::from_buff_direct(axml_buff, ChunkType::ResTablePackageType, package_chunk_start_pos)?;

        // This is the offset of the first field after the header (package_header.header_size)
        let package_body_start_offset = package_chunk_start_pos + package_header.header_size as u64;
        axml_buff.set_position(package_body_start_offset);

        // Get other members
        let id = axml_buff.read_u32::<LittleEndian>()?;

        let mut name_bytes: [u16; 128] = [0; 128];
        for i in 0..128 {
            name_bytes[i] = axml_buff.read_u16::<LittleEndian>()?;
        }
        // The name field in the struct is [u16; 128], so direct assignment is fine.

        let type_strings_offset_val = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_type = axml_buff.read_u32::<LittleEndian>()?;
        let key_strings_offset_val = axml_buff.read_u32::<LittleEndian>()?;
        let last_public_key = axml_buff.read_u32::<LittleEndian>()?;
        let type_id_offset = axml_buff.read_u32::<LittleEndian>()?;

        let current_pos_after_header_fields = axml_buff.position();

        // Parse Type String Pool
        let mut type_string_pool = None;
        if type_strings_offset_val != 0 {
            // The offset is from the start of the ResTable_package chunk itself (package_chunk_start_pos)
            let type_pool_abs_offset = package_chunk_start_pos + type_strings_offset_val as u64;
            axml_buff.set_position(type_pool_abs_offset);

            // We need to ensure ChunkType::parse_block_type and StringPool::from_buff
            // work correctly when called like this. StringPool::from_buff expects to parse
            // its own header after the type.
            let _type_pool_chunk_type = ChunkType::parse_block_type(axml_buff)?; // Should be ResStringPoolType
            if _type_pool_chunk_type != ChunkType::ResStringPoolType {
                return Err(AxmlError::UnexpectedChunk(format!("Expected ResStringPoolType for type strings, found {:?}", _type_pool_chunk_type)));
            }
            let mut strings = Vec::new();
            StringPool::from_buff(axml_buff, &mut strings)?; // from_buff now handles its own header.
            type_string_pool = Some(strings);
        }
        axml_buff.set_position(current_pos_after_header_fields); // Restore cursor to after package fixed fields

        // Parse Key String Pool
        let mut key_string_pool = None;
        if key_strings_offset_val != 0 {
            // The offset is from the start of the ResTable_package chunk itself (package_chunk_start_pos)
            let key_pool_abs_offset = package_chunk_start_pos + key_strings_offset_val as u64;
            axml_buff.set_position(key_pool_abs_offset);
            let _key_pool_chunk_type = ChunkType::parse_block_type(axml_buff)?; // Should be ResStringPoolType
             if _key_pool_chunk_type != ChunkType::ResStringPoolType {
                return Err(AxmlError::UnexpectedChunk(format!("Expected ResStringPoolType for key strings, found {:?}", _key_pool_chunk_type)));
            }
            let mut strings = Vec::new();
            StringPool::from_buff(axml_buff, &mut strings)?;
            key_string_pool = Some(strings);
        }

        // After parsing pools (if any), the cursor must be set to where the typeSpec/type chunks begin.
        // This is immediately after the ResTablePackage's fixed fields.
        // The fixed fields are: id, name, typeStrings, lastPublicType, keyStrings, lastPublicKey, typeIdOffset.
        // Their combined size is 4 + (128*2) + 4 + 4 + 4 + 4 + 4 = 280 bytes.
        // So, the next chunks start at package_body_start_offset + 280.
        // which is current_pos_after_header_fields.
        axml_buff.set_position(current_pos_after_header_fields);

        let mut type_specs = Vec::new();
        let mut types = Vec::new();

        let package_content_end_offset = package_chunk_start_pos + package_header.size as u64;

        while axml_buff.position() < package_content_end_offset {
            if package_content_end_offset - axml_buff.position() < 8 { // Min chunk header size
                break;
            }
            // Store position before reading type, so we can skip if unknown
            let chunk_read_start_pos = axml_buff.position();
            let chunk_type = ChunkType::parse_block_type(axml_buff)?;

            match chunk_type {
                ChunkType::ResTableTypeSpecType => {
                    let type_spec = ResTable_typeSpec::parse(axml_buff)?; // parse should handle cursor advancement
                    type_specs.push(type_spec);
                }
                ChunkType::ResTableTypeType => {
                    let type_entry = ResTable_type::parse(axml_buff)?; // parse should handle cursor advancement
                    types.push(type_entry);
                }
                _ => {
                    // An unknown chunk type. We need to skip it.
                    // We need its size. The header (type (2) + headerSize (2) + size (4)) = 8 bytes.
                    // We've read type (2 bytes). We need to read headerSize (2) and size (4).
                    axml_buff.set_position(chunk_read_start_pos); // Rewind to start of this unknown chunk
                    let _unknown_header = ChunkHeader::from_buff_no_type_check(axml_buff)?; // Reads the full header
                    // from_buff_no_type_check will advance the cursor by header.header_size
                    // The next step is to advance by chunk.size - chunk.header_size
                    let bytes_to_skip = _unknown_header.size - _unknown_header.header_size as u32;
                    if bytes_to_skip > 0 {
                         let mut skip_buf = vec![0; bytes_to_skip as usize];
                         axml_buff.read_exact(&mut skip_buf)?;
                    }
                    // Log this event: eprintln!("Skipped unknown chunk type {:?} of size {}", chunk_type, _unknown_header.size);
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
pub struct ResTable_config {
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

impl ResTable_config {
    /// Parse from a cursor of bytes
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let mut config = ResTable_config::default();
        let initial_cursor_pos = axml_buff.position(); // Position before reading anything from this specific struct

        config.size = axml_buff.read_u32::<LittleEndian>()?;

        // The payload starts after the 'size' field.
        // All offsets for fields are relative to the start of the payload.
        // config.size itself includes the 4 bytes of the size field.
        // So, a field at payload_offset X is readable if config.size >= 4 + payload_offset + field_size.

        // MCC (offset 0 from payload start, size 2)
        if config.size >= 4 + 0 + 2 {
            config.mcc = axml_buff.read_u16::<LittleEndian>()?;
        }
        // MNC (offset 2 from payload start, size 2)
        if config.size >= 4 + 2 + 2 {
            config.mnc = axml_buff.read_u16::<LittleEndian>()?;
        }
        // Language (offset 4 from payload start, size 2)
        if config.size >= 4 + 4 + 2 {
            axml_buff.read_exact(&mut config.language)?;
        }
        // Country (offset 6 from payload start, size 2)
        if config.size >= 4 + 6 + 2 {
            axml_buff.read_exact(&mut config.country)?;
        }
        // Orientation (offset 8 from payload start, size 1)
        if config.size >= 4 + 8 + 1 {
            config.orientation = axml_buff.read_u8()?;
        }
        // Touchscreen (offset 9 from payload start, size 1)
        if config.size >= 4 + 9 + 1 {
            config.touchscreen = axml_buff.read_u8()?;
        }
        // Density (offset 10 from payload start, size 2)
        if config.size >= 4 + 10 + 2 {
            config.density = axml_buff.read_u16::<LittleEndian>()?;
        }
        // Keyboard (offset 12 from payload start, size 1)
        if config.size >= 4 + 12 + 1 {
            config.keyboard = axml_buff.read_u8()?;
        }
        // Navigation (offset 13 from payload start, size 1)
        if config.size >= 4 + 13 + 1 {
            config.navigation = axml_buff.read_u8()?;
        }
        // InputFlags (offset 14 from payload start, size 1)
        if config.size >= 4 + 14 + 1 {
            config.input_flags = axml_buff.read_u8()?;
        }
        // InputPad0 (offset 15 from payload start, size 1)
        if config.size >= 4 + 15 + 1 {
            config.input_pad0 = axml_buff.read_u8()?;
        }
        // ScreenWidth (offset 16 from payload start, size 2)
        if config.size >= 4 + 16 + 2 {
            config.screen_width = axml_buff.read_u16::<LittleEndian>()?;
        }
        // ScreenHeight (offset 18 from payload start, size 2)
        if config.size >= 4 + 18 + 2 {
            config.screen_height = axml_buff.read_u16::<LittleEndian>()?;
        }
        // SdkVersion (offset 20 from payload start, size 2)
        if config.size >= 4 + 20 + 2 {
            config.sdk_version = axml_buff.read_u16::<LittleEndian>()?;
        }
        // MinorVersion (offset 22 from payload start, size 2)
        if config.size >= 4 + 22 + 2 { // This field is at payload offset 22, total header size up to here is 26
            config.minor_version = axml_buff.read_u16::<LittleEndian>()?;
        }

        // Fields from ResTable_config_v1 (Android 1.6+)
        // screenLayout (offset 24 from payload start, size 1). Minimum struct size: 4+24+1 = 29 (historically 28, but Android uses size for this)
        if config.size >= 28 && config.size >= 4 + 24 + 1 { // ResTable_config.h shows this starts at size 28
            config.screen_layout = axml_buff.read_u8()?;
        }
        // uiMode (offset 25 from payload start, size 1). Minimum struct size: 4+25+1 = 30 (historically 29)
        if config.size >= 28 && config.size >= 4 + 25 + 1 { // Android uses size 28 as base for this too
             config.ui_mode = axml_buff.read_u8()?;
        }
        // smallestScreenWidthDp (offset 26 from payload start, size 2). Minimum struct size: 4+26+2 = 32 (historically 31)
        if config.size >= 32 && config.size >= 4 + 26 + 2 { // Android uses size 32
            config.smallest_screen_width_dp = axml_buff.read_u16::<LittleEndian>()?;
        }

        // Fields from ResTable_config_v2 (Android 3.2+)
        // screenWidthDp (offset 28 from payload start, size 2). Minimum struct size: 4+28+2 = 34 (historically 33)
        if config.size >= 36 && config.size >= 4 + 28 + 2 { // Android uses size 36
            config.screen_width_dp = axml_buff.read_u16::<LittleEndian>()?;
        }
        // screenHeightDp (offset 30 from payload start, size 2). Minimum struct size: 4+30+2 = 36 (historically 35)
        if config.size >= 36 && config.size >= 4 + 30 + 2 {
            config.screen_height_dp = axml_buff.read_u16::<LittleEndian>()?;
        }

        // Fields from ResTable_config_v3 (Android 4.2+)
        // localeScript (offset 32 from payload start, size 4). Minimum struct size: 4+32+4 = 40 (historically 39)
        if config.size >= 40 && config.size >= 4 + 32 + 4 { // Android uses size 40 for locale script
            axml_buff.read_exact(&mut config.locale_script)?;
        }
        // localeVariant (offset 36 from payload start, size 8). Minimum struct size: 4+36+8 = 48 (historically 47)
        if config.size >= 48 && config.size >= 4 + 36 + 8 { // Android uses size 48 for locale variant
            axml_buff.read_exact(&mut config.locale_variant)?;
        }

        // screenLayout2 (offset 44 from payload start, size 1). Minimum struct size: 4+44+1 = 49 (historically 48)
        // This field was added in API level 17 (JELLY_BEAN_MR1)
        // The size check should be for the specific version of ResTable_config that includes this.
        // AOSP ResourceTypes.h defines various sizes for different versions.
        // For simplicity here, we use the specific size checks as per Android's `ResTable_config::sizeForConfig`.
        // Size 48: up to localeVariant.
        // Size 52: adds screenLayout2, screenPad2, screenPad3. (Actually screenLayout2 and screenPad2 are at size 48, screenPad3 is for API 21 / size 52)
        // Let's use the documented offsets.
        // screenLayout2 is at offset 44 of payload. Total size needed: 4 + 44 + 1 = 49 bytes.
        // Android's `size` for this configuration (with screenLayout2) is typically 48 (if no further fields).
        // Let's stick to what the `size` field indicates.
        if config.size >= 4 + 44 + 1 { // Field at offset 44
            config.screen_layout2 = axml_buff.read_u8()?;
        }
        // screenPad2 (offset 45 from payload start, size 1)
        if config.size >= 4 + 45 + 1 { // Field at offset 45
             config.screen_pad2 = axml_buff.read_u8()?;
        }
        // screenPad3 (offset 46 from payload start, size 2)
         if config.size >= 4 + 46 + 2 { // Field at offset 46
            config.screen_pad3 = axml_buff.read_u16::<LittleEndian>()?;
        }

        // Ensure the cursor is advanced to the end of the declared size of this config struct
        axml_buff.set_position(initial_cursor_pos + config.size as u64);

        Ok(config)
    }
}
