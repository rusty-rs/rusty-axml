pub mod parser;
pub mod chunks;
pub mod errors;

use std::{
    fs,
    collections::HashMap,
};
use std::io::{
    Read,
    Cursor,
};
use std::rc::Rc;
use std::cell::RefCell;

use crate::chunks::{
    resource_map::ResourceMap,
    res_table::ResTable,
    string_pool::StringPool,
};
use crate::parser::{
    Axml,
    XmlElement
};

/// Representation of an app's manifest contents
#[derive(Debug, Default)]
pub struct ManifestContents {
    pub pkg_name: String,

    pub activities: Vec<String>,
    pub services: Vec<String>,
    pub providers: Vec<String>,
    pub receivers: Vec<String>,

    // TODO: does not includes permissions requested from within components
    pub created_perms: Vec<String>,
    pub requested_perms: Vec<String>,

    pub main_entry_point: Option<String>,
}

/// A component can be exported or enabled. Each of these feature have default values
/// but these default values can be overriden by the developer. This means they have
/// essentially four states:
///     * default to `true`,
///     * default to `false`,
///     * explicitely set to `true`,
///     * explicitely set to `false`
#[derive(Debug, PartialEq)]
pub enum ComponentState {
    Unknown,
    DefaultTrue,
    DefaultFalse,
    ExplicitTrue,
    ExplicitFalse,
}

/// Open an APK, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
/// This function expects `file_path` to point to an APK (or really, any valid
/// zip file that contains a file named `AndroidManifest.xml`).
/// To read an AXML file directly use [`create_cursor_from_axml`] instead.
///
/// [`create_cursor_from_axml`]: fn.create_cursor_from_axml.html
pub fn create_cursor_from_apk(file_path: &str) -> Cursor<Vec<u8>> {

    let mut axml_cursor = Vec::new();

    let zipfile = std::fs::File::open(file_path).unwrap();
    let mut archive = zip::ZipArchive::new(zipfile).unwrap();
    let mut raw_file = match archive.by_name("AndroidManifest.xml") {
        Ok(file) => file,
        Err(..) => {
            panic!("Error: no AndroidManifest.xml in APK");
        }
    };
    raw_file.read_to_end(&mut axml_cursor).expect("Error: cannot read manifest from app");

    Cursor::new(axml_cursor)
}

/// Open an AXML file, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
/// This function expects `file_path` to point to an AXML file.
/// To read the manifest from an APK file use [`create_cursor_from_apk`] instead.
///
/// [`create_cursor_from_apk`]: fn.create_cursor_from_apk.html
pub fn create_cursor_from_axml(file_path: &str) -> Cursor<Vec<u8>> {

    let mut axml_cursor = Vec::new();

    let mut raw_file = fs::File::open(file_path).expect("Error: cannot open AXML file");
    raw_file.read_to_end(&mut axml_cursor).expect("Error: cannot read AXML file");

    Cursor::new(axml_cursor)
}

/// Parses the AXML file from a cursor.
///
/// This function will return an `XmlElement` which represents the root of the
/// parsed XML document. Note that the Cursor must first be created using either
/// [`create_cursor_from_axml`] or [`create_cursor_from_apk`].
///
/// [`create_cursor_from_axml`]: fn.create_cursor_from_axml.html
/// [`create_cursor_from_apk`]: fn.create_cursor_from_apk.html
pub fn parse_from_cursor(axml_cursor: Cursor<Vec<u8>>) -> Axml {
    match parser::parse_xml(axml_cursor) {
        Ok(axml) => axml,
        Err(err) => panic!("{err}")
    }
}

/// Parses the AXML file from a string.
///
/// Given a string that represents an AXML document, parses the string into XML.
/// This function will return an `XmlElement` which represents the root of the
/// parsed XML document.
pub fn parse_from_string(axml_str: &str) -> Axml {
    let axml_cursor = Cursor::new(Vec::from(axml_str.as_bytes()));
    match parser::parse_xml(axml_cursor) {
        Ok(axml) => axml,
        Err(err) => panic!("{err}")
    }
}

/// Parses the AXML from a generic reader
///
/// This is a more generic version of [`parse_from_string`]. This function will
/// read and attempt to parse AXML from any type that implements the [`Read`] trait.
///
/// [`parse_from_string`]: fn.parse_from_string.html
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub fn parse_from_reader<R>(mut reader: R) -> Axml
where
    R: Read,
{
    // TODO: properly handle errors while reading
    let mut axml_vec = Vec::<u8>::new();
    if let Err(err) = reader.read_to_end(&mut axml_vec) {
        panic!("Error: cannot read bytes from reader: {err}");
    }

    let axml_cursor = Cursor::new(axml_vec);
    match parser::parse_xml(axml_cursor) {
        Ok(axml) => axml,
        Err(err) => panic!("{err}")
    }
}

/// Use BFS tree traversal to get all element of a given type
///
/// TODO: use Axml type here instead
fn find_elements_by_type(parsed_xml: &Rc<RefCell<XmlElement>>, element_type: &str) -> Vec<Rc<RefCell<XmlElement>>> {
    let mut result = Vec::new();
    let mut stack = vec![Rc::clone(parsed_xml)];

    while let Some(element) = stack.pop() {
        let borrowed = element.borrow();
        if borrowed.element_type == element_type {
            result.push(Rc::clone(&element));
        }
        for child in &borrowed.children {
            stack.push(Rc::clone(child));
        }
    }

    result
}

/// Check if a component is exposed which is the case if it is both enabled and exported
/// Both of these properties can either be explicitely set (as parameters in the compoennt
/// declaration in the manifest) or left to their default state.
/// The default state depends on the presence or not of intent filters: if there is an intent
/// filter, the assumption is that the compoennt is meants to be available to other apps, and so it
/// is exported by default, otherwise not.
fn is_component_exposed(component: &Rc<RefCell<XmlElement>>) -> bool {
    let mut _enabled_state = ComponentState::DefaultTrue;
    let mut exported_state = ComponentState::Unknown;

    if let Some(enabled) = component.borrow().attributes.get("android:enabled") {
        if enabled == "false" {
            return false;
        } else {
            _enabled_state = ComponentState::ExplicitTrue;
        }
    }

    if let Some(exported) = component.borrow().attributes.get("android:exported") {
        if exported == "false" {
            return false;
        } else {
            exported_state = ComponentState::ExplicitTrue;
        }
    }

    // If the component has intent filters then the default exported value is `true`, otherwise
    // `false`. This is not the case for content providers though, which usually have explicit
    // values anyway.
    if exported_state == ComponentState::Unknown {
        for item in component.borrow().children.iter() {
            if item.borrow().element_type == "intent-filter" {
                exported_state = ComponentState::DefaultTrue;
                break;
            }
        }
        if exported_state == ComponentState::Unknown {
            exported_state = ComponentState::DefaultFalse;
        }
    }

    // At this point we know the component is enabled so we just need to check if it is also
    // exported. Also, if the component is explicitly not exported then we return early so here we
    // do not have to check all the cases
    match exported_state {
        ComponentState::DefaultFalse => false,
        ComponentState::DefaultTrue => true,
        ComponentState::ExplicitTrue => true,
        _ => panic!("never going to happen")
    }
}

/// Parse an app's manifest and get the list of exposed components
/// We first check if the app has the `android:enabled` component set, which would influence the
/// state of all the components in the app
pub fn get_exposed_components(parsed_xml: Rc<RefCell<XmlElement>>) -> Option<HashMap<String, Vec<Rc<RefCell<XmlElement>>>>> {
    // Checking if the `<application>` tag has the `enabled` attribute set to `false`
    let application = find_elements_by_type(&parsed_xml, "application").pop()?;
    if let Some(enabled) = application.borrow().attributes.get("android:enabled") {
        if enabled == "false" {
            return None;
        }
    }

    let mut components = HashMap::new();

    components.insert(
        String::from("activity"),
        find_elements_by_type(&parsed_xml, "activity")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("service"),
        find_elements_by_type(&parsed_xml, "service")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("provider"),
        find_elements_by_type(&parsed_xml, "provider")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("receiver"),
        find_elements_by_type(&parsed_xml, "receiver")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );

    Some(components)
}

