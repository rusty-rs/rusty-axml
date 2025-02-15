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
    StringDecodeError(#[from] std::char::DecodeUtf16Error),
    #[error("string not in the string pool")]
    StringPoolError,
    #[error("unknown namespace")]
    NamespaceError,
    #[error("zip file error")]
    ZipFileError(#[from] zip::result::ZipError)
}
