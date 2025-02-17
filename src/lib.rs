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

use crate::errors::AxmlError;
use crate::chunks::{
    resource_map::ResourceMap,
    res_table::ResTable,
    string_pool::StringPool,
};
use crate::parser::{
    Axml,
    XmlNode
};

/// Component state
///
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

/// Read and parse the manifest of an APK
///
/// This function will extract the binary AXML from an APK and parse it, returning
/// an `Axml` object.
///
/// # Example
///
/// ```
/// let axml = rusty_axml::parse_from_apk("tests/assets/app.apk").unwrap();
/// assert!(rusty_axml::get_requested_permissions(&axml)
///                    .contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()));
/// ```
pub fn parse_from_apk(file_path: &str) -> Result<Axml, AxmlError> {
    let cursor = create_cursor_from_apk(file_path)?;
    parse_from_cursor(cursor)
}

/// Read and parse an AXML file
///
/// This function will read a binary AXML and parse it, returning an `Axml` object.
///
/// # Example
///
/// ```
/// let axml = rusty_axml::parse_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// assert!(rusty_axml::get_requested_permissions(&axml)
///                    .contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()));
/// ```
pub fn parse_from_axml(file_path: &str) -> Result<Axml, AxmlError> {
    let cursor = create_cursor_from_axml(file_path)?;
    parse_from_cursor(cursor)
}

/// Create cursor of bytes from an APK
///
/// Open an APK, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
/// This function expects `file_path` to point to an APK (or really, any valid
/// zip file that contains a file named `AndroidManifest.xml`).
/// To read an AXML file directly use [`create_cursor_from_axml`] instead.
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_apk("tests/assets/app.apk").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert!(rusty_axml::get_requested_permissions(&axml)
///                    .contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()))
/// ```
///
/// [`create_cursor_from_axml`]: fn.create_cursor_from_axml.html
pub fn create_cursor_from_apk(file_path: &str) -> Result<Cursor<Vec<u8>>, AxmlError> {
    let mut axml_cursor = Vec::new();

    let zipfile = std::fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(zipfile)?;
    let mut raw_file = match archive.by_name("AndroidManifest.xml") {
        Ok(file) => file,
        Err(..) => {
            panic!("Error: no AndroidManifest.xml in APK");
        }
    };
    raw_file.read_to_end(&mut axml_cursor)?;

    Ok(Cursor::new(axml_cursor))
}

/// Create cursor of bytes from an AXML file
///
/// Open an AXML file, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
/// This function expects `file_path` to point to an AXML file.
/// To read the manifest from an APK file use [`create_cursor_from_apk`] instead.
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert!(rusty_axml::get_requested_permissions(&axml)
///                    .contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()))
/// ```
///
/// [`create_cursor_from_apk`]: fn.create_cursor_from_apk.html
pub fn create_cursor_from_axml(file_path: &str) -> Result<Cursor<Vec<u8>>, AxmlError> {
    let mut axml_cursor = Vec::new();

    let mut raw_file = fs::File::open(file_path)?;
    raw_file.read_to_end(&mut axml_cursor)?;

    Ok(Cursor::new(axml_cursor))
}

/// Parses the AXML file from a cursor.
///
/// This function will return an `XmlElement` which represents the root of the
/// parsed XML document. Note that the Cursor must first be created using either
/// [`create_cursor_from_axml`] or [`create_cursor_from_apk`].
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert!(rusty_axml::get_requested_permissions(&axml)
///                    .contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()))
/// ```
///
/// [`create_cursor_from_axml`]: fn.create_cursor_from_axml.html
/// [`create_cursor_from_apk`]: fn.create_cursor_from_apk.html
pub fn parse_from_cursor(axml_cursor: Cursor<Vec<u8>>) -> Result<Axml, AxmlError> {
    parser::parse_xml(axml_cursor)
}

/// Parses the AXML file from a string.
///
/// Given a string that represents an AXML document, parses the string into XML.
/// This function will return an `XmlElement` which represents the root of the
/// parsed XML document.
pub fn parse_from_string(axml_str: &str) -> Result<Axml, AxmlError> {
    let axml_cursor = Cursor::new(Vec::from(axml_str.as_bytes()));
    parser::parse_xml(axml_cursor)
}

/// Parses the AXML from a generic reader
///
/// This is a more generic version of [`parse_from_string`]. This function will
/// read and attempt to parse AXML from any type that implements the [`Read`] trait.
///
/// [`parse_from_string`]: fn.parse_from_string.html
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub fn parse_from_reader<R>(mut reader: R) -> Result<Axml, AxmlError>
where
    R: Read,
{
    let mut axml_vec = Vec::<u8>::new();
    reader.read_to_end(&mut axml_vec)?;

    let axml_cursor = Cursor::new(axml_vec);
    parser::parse_xml(axml_cursor)
}

/// Return all elements of the given type
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::find_nodes_by_type(&axml, "activity").len(), 1)
/// ```
pub fn find_nodes_by_type(axml: &Axml, element_type: &str) -> Vec<XmlNode> {
    axml.iter()
        .filter(|element| element.borrow().element_type() == element_type)
        .collect()
}

/// Returns an `XmlNode` if it exists
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// let service = rusty_axml::find_node_by_name(&axml, "androidx.profileinstaller.ProfileInstallReceiver").unwrap();
/// assert_eq!(service.borrow().get_attr("android:permission"), Some("android.permission.DUMP"));
/// ```
pub fn find_node_by_name(axml: &Axml, node_name: &str) -> Option<XmlNode> {
    let name = node_name.to_string();

    // Component names are unique so after filter there will either be zero or one element
    // Calling next() will either yield the component or `None`
    axml.iter()
        .find(|element| element.borrow().get_name() == Some(&name))
}

/// Check if a component is exposed
///
/// A component is considered exposed if it is both enabled and exported. Both of these properties
/// can either be explicitely set (as parameters in the component declaration in the manifest) or
/// left to their default state. The default state depends on the presence or not of intent
/// filters: if there is an intent filter, the assumption is that the compoennt is meants to be
/// available to other apps, and so it is exported by default, otherwise not.
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// let service = rusty_axml::find_node_by_name(&axml, "eu.jgamba.myapplication.MyFirstService").unwrap();
/// assert!(rusty_axml::is_component_exposed(&service));
/// ```
pub fn is_component_exposed(component: &XmlNode) -> bool {
    let mut _enabled_state = ComponentState::DefaultTrue;
    let mut exported_state = ComponentState::Unknown;

    if let Some(enabled) = component.borrow().get_attr("android:enabled") {
        if enabled == "false" {
            return false;
        } else {
            _enabled_state = ComponentState::ExplicitTrue;
        }
    }

    if let Some(exported) = component.borrow().get_attr("android:exported") {
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
        for item in component.borrow().children().iter() {
            if item.borrow().element_type() == "intent-filter" {
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

/// Get the list of activities names
///
/// This is only valid for APK manifest files and will return an empty vector otherwise
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_activities_names(&axml).len(), 1)
/// ```
pub fn get_activities_names(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "activity")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Get the list of services names
///
/// This is only valid for APK manifest files and will return an empty vector otherwise
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_services_names(&axml).len(), 2)
/// ```
pub fn get_services_names(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "service")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Get the list of providers names
///
/// This is only valid for APK manifest files and will return an empty vector otherwise
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_providers_names(&axml).len(), 2)
/// ```
pub fn get_providers_names(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "provider")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Get the list of receivers names
///
/// This is only valid for APK manifest files and will return an empty vector otherwise
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_receivers_names(&axml).len(), 2)
/// ```
pub fn get_receivers_names(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "receiver")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Get the list of declared permissions
///
/// This is only valid for APK manifest files and will return an empty vector otherwise.
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_declared_permissions(&axml).len(), 2)
/// ```
pub fn get_declared_permissions(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "permission")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Get the list of requested permissions
///
/// This is only valid for APK manifest files and will return an empty vector otherwise. This also
/// does not include permissions requested from within components.
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// assert_eq!(rusty_axml::get_requested_permissions(&axml).len(), 3)
/// ```
pub fn get_requested_permissions(parsed_xml: &Axml) -> Vec<String> {
    find_nodes_by_type(parsed_xml, "uses-permission")
        .into_iter()
        .filter(|element| element.borrow().get_name().is_some())
        .map(|element| element.borrow().get_name().unwrap().to_string())
        .collect()
}

/// Parse an app's manifest and get the list of exposed components
///
/// We first check if the app has the `android:enabled` component set, which would influence the
/// state of all the components in the app
///
/// # Example
///
/// ```
/// let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
/// let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
///
/// let exposed_components = rusty_axml::get_exposed_components(&axml).unwrap();
/// assert_eq!(exposed_components.get("activity").unwrap().len(), 1);
/// assert_eq!(exposed_components.get("service").unwrap().len(), 1);
/// assert_eq!(exposed_components.get("receiver").unwrap().len(), 2);
/// assert_eq!(exposed_components.get("provider").unwrap().len(), 0);
/// ```
pub fn get_exposed_components(parsed_xml: &Axml) -> Option<HashMap<String, Vec<XmlNode>>> {
    // Checking if the `<application>` tag has the `enabled` attribute set to `false`
    let application = find_nodes_by_type(parsed_xml, "application").pop()?;
    if let Some(enabled) = application.borrow().get_attr("android:enabled") {
        if enabled == "false" {
            return None;
        }
    }

    let mut components = HashMap::new();

    components.insert(
        String::from("activity"),
        find_nodes_by_type(parsed_xml, "activity")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("service"),
        find_nodes_by_type(parsed_xml, "service")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("provider"),
        find_nodes_by_type(parsed_xml, "provider")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );
    components.insert(
        String::from("receiver"),
        find_nodes_by_type(parsed_xml, "receiver")
                .into_iter()
                .filter(is_component_exposed)
                .collect()
    );

    Some(components)
}

