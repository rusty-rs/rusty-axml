//! Errors module
//!
//! This module simply enumerates the possible errors we can
//! encounter and their associated error messages. We use
//! `thiserror` for the heavy lifting.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AxmlError {
    #[error("reading from the cursor of bytes")]
    IoError(#[from] std::io::Error),
    #[error("cannot decode string from UTF-16")]
    Utf16DecodeError(#[from] std::char::DecodeUtf16Error),
    #[error("cannot decode string from UTF-8")]
    Utf8StrDecodeError(#[from] std::str::Utf8Error),
    #[error("cannot decode string from UTF-8")]
    Utf8StringDecodeError(#[from] std::string::FromUtf8Error),
    #[error("string not in the string pool")]
    StringPoolError,
    #[error("unknown namespace")]
    NamespaceError,
    #[error("Zip file error")]
    ZipFileError(#[from] zip::result::ZipError),
    #[error("XML encoding error")]
    XmlEncodingError(#[from] quick_xml::Error),
}
