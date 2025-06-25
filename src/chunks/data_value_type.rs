#![allow(dead_code)]

//! Data value types
//!
//! Possible data type values in Dalvik and a helper method to parse a data type from an `u8`.

use std::fmt::{
    Formatter,
    LowerHex
};

/// Data value types
///
/// Note: we ignore `TypeFirstInt`, `TypeFirstColorInt`, and `TypeLastColorInt` which hold the same values
/// as actual data types (respectively `TypeIntDec`, `TypeIntColorArgb8`, and `TypeIntColorRgb4`).
#[derive(Debug)]
pub enum DataValueType {
    /// The 'data' is either 0 or 1, specifying this resource is either undefined or empty,
    ///respectively
    TypeNull	            = 0x00,
    /// The 'data' holds a ResTable_ref, a reference to another resource table entry
    TypeReference	        = 0x01,
    /// The 'data' holds an attribute resource identifier
    TypeAttribute	        = 0x02,
    /// The 'data' holds an index into the containing resource table's global value string pool
    TypeString	            = 0x03,
    /// The 'data' holds a single-precision floating point number
    TypeFloat	            = 0x04,
    /// The 'data' holds a complex number encoding a dimension value, such as "100in"
    TypeDimension	        = 0x05,
    /// The 'data' holds a complex number encoding a fraction of a container
    TypeFraction	        = 0x06,
    /// The 'data' holds a dynamic ResTable_ref, which needs to be resolved before it can be used
    /// like a TYPE_REFERENCE
    TypeDynamicReference    = 0x07,
    /// The 'data' holds an attribute resource identifier, which needs to be resolved before it can
    /// be used like a TYPE_ATTRIBUTE
    TypeDynamicAttribute	= 0x08,

    /// Integers
    ///
    /// The data is a raw integer value of the form `n..n`
    TypeIntDec	            = 0x10,
    /// The data is a raw integer value of the form `0xn..n`
    TypeIntHex	            = 0x11,
    /// The data is either 0 or 1, for input `false` or `true` respectively
    TypeIntBoolean	        = 0x12,

    /// Colors
    ///
    /// The 'data' is a raw integer value of the form `#aarrggbb`
    TypeIntColorArgb8		= 0x1c,
    /// The 'data' is a raw integer value of the form `#rrggbb`
    TypeIntColorRgb8		= 0x1d,
    /// The 'data' is a raw integer value of the form `#argb`
    TypeIntColorArgb4		= 0x1e,
    /// The 'data' is a raw integer value of the form `#rgb`
    TypeIntColorRgb4		= 0x1f,
}

impl DataValueType {
    /// Attempt to convert `u8` into a `DataValueType` 
    pub fn from_val(value: u8) -> Self {
        match value {
            0x00 => DataValueType::TypeNull,
            0x01 => DataValueType::TypeReference,
            0x02 => DataValueType::TypeAttribute,
            0x03 => DataValueType::TypeString,
            0x04 => DataValueType::TypeFloat,
            0x05 => DataValueType::TypeDimension,
            0x06 => DataValueType::TypeFraction,
            0x07 => DataValueType::TypeDynamicReference,
            0x08 => DataValueType::TypeDynamicAttribute,
            0x10 => DataValueType::TypeIntDec,
            0x11 => DataValueType::TypeIntHex,
            0x12 => DataValueType::TypeIntBoolean,
            0x1c => DataValueType::TypeIntColorArgb8,
            0x1d => DataValueType::TypeIntColorRgb8,
            0x1e => DataValueType::TypeIntColorArgb4,
            0x1f => DataValueType::TypeIntColorRgb4,
            _ => panic!("Error: unknown data value type {:02X}", value)
        }
    }
}

impl LowerHex for DataValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            DataValueType::TypeNull             => LowerHex::fmt(&0x00, f),
            DataValueType::TypeReference        => LowerHex::fmt(&0x01, f),
            DataValueType::TypeAttribute        => LowerHex::fmt(&0x02, f),
            DataValueType::TypeString           => LowerHex::fmt(&0x03, f),
            DataValueType::TypeFloat            => LowerHex::fmt(&0x04, f),
            DataValueType::TypeDimension        => LowerHex::fmt(&0x05, f),
            DataValueType::TypeFraction         => LowerHex::fmt(&0x06, f),
            DataValueType::TypeDynamicReference => LowerHex::fmt(&0x07, f),
            DataValueType::TypeDynamicAttribute => LowerHex::fmt(&0x08, f),
            DataValueType::TypeIntDec           => LowerHex::fmt(&0x10, f),
            DataValueType::TypeIntHex           => LowerHex::fmt(&0x11, f),
            DataValueType::TypeIntBoolean       => LowerHex::fmt(&0x12, f),
            DataValueType::TypeIntColorArgb8    => LowerHex::fmt(&0x1c, f),
            DataValueType::TypeIntColorRgb8     => LowerHex::fmt(&0x1d, f),
            DataValueType::TypeIntColorArgb4    => LowerHex::fmt(&0x1e, f),
            DataValueType::TypeIntColorRgb4     => LowerHex::fmt(&0x1f, f),
        }
    }
}

/// Where the unit type information is for complex values.
/// This gives us 16 possible types, as defined below.
const COMPLEX_UNIT_SHIF: u8 = 0;
const COMPLEX_UNIT_MASK: u8 = 0xf;

/// Structure of complex unit data values
#[derive(Debug)]
pub enum ComplexValueUnitType {
    /// TYPE_DIMENSION: Value is raw pixels.
    ComplexUnitPx = 0,
    /// TYPE_DIMENSION: Value is Device Independent Pixels.
    ComplexUnitDip = 1,
    /// TYPE_DIMENSION: Value is a Scaled device independent Pixels.
    ComplexUnitSp = 2,
    /// TYPE_DIMENSION: Value is in points.
    ComplexUnitPt = 3,
    /// TYPE_DIMENSION: Value is in inches.
    ComplexUnitIn = 4,
    /// TYPE_DIMENSION: Value is in millimeters.
    ComplexUnitMm = 5,
}

/// TYPE_FRACTION: A basic fraction of the overall size.
const COMPLEX_UNIT_FRACTION: u8 = 0;
/// TYPE_FRACTION: A fraction of the parent size.
const COMPLEX_UNIT_FRACTION_PARENT: u8 = 1;

/// Where the radix information is, telling where the decimal place
/// appears in the mantissa.  This give us 4 possible fixed point
/// representations as defined below.
const COMPLEX_RADIX_SHIFT: u8 = 4;
const COMPLEX_RADIX_MASK: u8 = 0x3;

/// Where the actual value is.  This gives us 23 bits of
/// precision.  The top bit is the sign.
const COMPLEX_MANTISSA_SHIFT: u8 = 8;
const COMPLEX_MANTISSA_MASK: u32 = 0xffffff;

/// Structure of fraction complex data values
#[derive(Debug)]
pub enum ComplexValueFractionType {
    /// The mantissa is an integral number -- i.e., 0xnnnnnn.0
    ComplexRadix23p0 = 0,
    /// The mantissa magnitude is 16 bits -- i.e, 0xnnnn.nn
    ComplexRadix16p7 = 1,
    /// The mantissa magnitude is 8 bits -- i.e, 0xnn.nnnn
    ComplexRadix8p15 = 2,
    /// The mantissa magnitude is 0 bits -- i.e, 0x0.nnnnnn
    ComplexRadix0p23 = 3,
}

/// Possible values for `TYPE_NULL`
#[derive(Debug)]
pub enum DataNull {
    /// The value is not defined.
    DataNullUndefined = 0,
    /// The value is explicitly defined as empty.
    DataNullEmpty = 1
}
