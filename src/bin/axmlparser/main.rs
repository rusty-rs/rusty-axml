#![cfg(feature = "cli")]
pub mod cli;

use std::fs::File;
use rusty_axml::{
    create_cursor_from_apk,
    create_cursor_from_axml,
};
use rusty_axml::parser;

use crate::cli::ArgType;

fn main() {
    // Check CLI arguments
    let args = cli::parse_args();

    // Check the file type
    let arg_type = args.get_arg_type();
    let arg_path = args.get_arg_path();

    // Create cursor over input file contents
    let axml_cursor = match arg_type {
        ArgType::Apk  => { create_cursor_from_apk(&arg_path)  },
        ArgType::Axml => { create_cursor_from_axml(&arg_path) },
        _ => todo!()
    };

    // Parse the XML
    let elements = parser::parse_xml(axml_cursor);

    // Write to file if `args::output` is not `None`
    if let Some(opath) = args.get_output_path() {
        let mut ofile = File::create(opath)
            .expect("Error: cannot open file {opath}");
        elements.borrow().write_to_file(&mut ofile)
            .expect("Error: cannot write to file {opath}");
    };
}
