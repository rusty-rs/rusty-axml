//! Errors module
//!
//! This module simply enumerates the possible errors we can
//! encounter and their associated error messages. We use
//! `thiserror` for the heavy lifting.

/*
use std::fmt::Display;

impl Display for AxmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match AxmlErro
    }
}
*/

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AxmlError {
    #[error("reading from the cursor of bytes")]
    CursorReadError(#[from] std::io::Error),
    #[error("string not in the string pool")]
    StringPoolError,
}
