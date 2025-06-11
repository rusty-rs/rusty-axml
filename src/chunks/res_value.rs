#![allow(dead_code)]

use crate::{
    chunks::data_value_type::DataValueType,
    errors::AxmlError
};

use std::io:: Cursor;
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

/// Representation of a value in a resource, supplying type information.
#[derive(Debug, PartialEq)]
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
    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, AxmlError> {
        let size = axml_buff.read_u16::<LittleEndian>()?;
        let res0 = axml_buff.read_u8()?;

        if res0 != 0 {
            panic!("res0 is not 0");
        }

        let data_type = DataValueType::from_val(axml_buff.read_u8()?);
        let data = axml_buff.read_u32::<LittleEndian>()?;

        Ok(ResValue {
            size,
            res0,
            data_type,
            data
        })
    }
}
