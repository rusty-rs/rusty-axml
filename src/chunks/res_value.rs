#![allow(dead_code)]

use crate::chunks::data_value_type::DataValueType;

use std::io::{
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

/// Representation of a value in a resource, supplying type information.
pub struct ResValue {
    /// Number of bytes in this structure
    pub size: u16,

    /// Always set to 0
    pub res0: u8,

    /// The type of data
    pub data_type: DataValueType,

    /// The actual data
    pub data: u32,
}

impl ResValue {
    /// Parse from a cursor of bytes
    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        let size = axml_buff.read_u16::<LittleEndian>().unwrap();
        let res0 = axml_buff.read_u8().unwrap();

        if res0 != 0 {
            panic!("res0 is not 0");
        }

        let data_type = DataValueType::from_val(axml_buff.read_u8().unwrap());
        let data = axml_buff.read_u32::<LittleEndian>().unwrap();

        Ok(ResValue {
            size,
            res0,
            data_type,
            data
        })
    }
}
