#![cfg(feature = "cli")]
pub mod cli;

use rusty_axml::create_cursor_from_apk;
use rusty_axml::parser;

fn main() {
    // Check CLI arguments
    let args = cli::parse_args();

    // Check the file type
    let arg_path = args.get_arg_path();

    // Create cursor over input file contents
    let axml_cursor = create_cursor_from_apk(&arg_path);

    // Parse the XML
    let elements = parser::parse_xml(axml_cursor);
    println!("{elements:?}");

    // TODO: convert into actual AXML and offer
    // the possibility to write it to a file
}
