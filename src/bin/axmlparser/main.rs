#![cfg(feature = "cli")]
pub mod cli;

use std::fs::File;
use rusty_axml::{
    create_cursor_from_apk,
    create_cursor_from_axml,
};

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
    let elements = rusty_axml::parse_from_cursor(axml_cursor);

    // Write to file if `args::output` is not `None`
    match args.get_output_path() {
        Some(opath) => {
            let mut ofile = File::create(opath)
                .expect("Error: cannot open file {opath}");
            elements.borrow().write_to_file(&mut ofile)
                .expect("Error: cannot write to file {opath}");
        },
        None => {
            if let Ok(str_xml) = elements.borrow().to_string() {
                println!("{str_xml}");
            };
        }
    }
}
