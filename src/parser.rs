//! Main parser routine
//!
//! This module contains the logic to parse the binary XML into a tree structure (`XmlElement`),
//! representing the actual XML.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{
    Error,
    Cursor,
    Write,
};
use std::fs::File;

use byteorder::{
    LittleEndian,
    ReadBytesExt
};

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, Event};

use crate::errors::AxmlError;
use crate::{
    ResourceMap,
    StringPool,
    ResTable
};

use crate::chunks::{
    chunk_types::ChunkType,
    chunk_header::ChunkHeader,
    data_value_type::DataValueType,
    res_value::ResValue,
};

/// Representation of an XML element with optional children
#[derive(Debug)]
pub struct XmlElement {
    /// Type of element (e.g., `activity`, `service`)
    element_type: String,
    /// Attributes of the element (e.g., `exported`, `permission`)
    attributes: HashMap<String, String>,
    /// Vector of children of the XML element
    children: Vec<XmlNode>
}

impl XmlElement {
    /// Write an `XmlElement` into a writer
    fn write_element<W: Write>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        let mut element = writer.create_element(&self.element_type);

        element = if self.attributes.is_empty() {
            element
        } else {
            element.with_attributes(
                self.attributes
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<(&str, &str)>>(),
            )
        };

        if self.children.is_empty() {
            element.write_empty().unwrap();
        } else {
            element
                .write_inner_content(|writer| -> Result<(), quick_xml::Error> {
                    for child in self.children.iter() {
                        child.as_ref().borrow().write_element(writer).unwrap();
                    }

                    Ok(())
                })
                .unwrap();
        }

        Ok(())
    }

    /// Get the element's type
    pub fn element_type(&self) -> &str {
        &self.element_type
    }

    /// Get the element's children
    pub fn children(&self) -> &[XmlNode] {
        &self.children
    }

    /// Get the element's attributes
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Get the element's name off of its attributes if it exists
    pub fn get_name(&self) -> Option<&str> {
        if let Some(attr) = self.attributes.get("android:name") {
            return Some(attr);
        }

        None
    }

    /// Get an attribute from an `XmlElement` if it exists
    pub fn get_attr(&self, attr_name: &str) -> Option<&str> {
        if let Some(attr) = self.attributes.get(attr_name) {
            return Some(attr);
        }

        None
    }
}

/// XML nodes
pub type XmlNode = Rc<RefCell<XmlElement>>;

/// Representation of the whole XML document
#[derive(Debug)]
pub struct Axml {
    /// Root of the XML doc
    root: XmlNode,
}

impl Axml {
    /// Write the whole parsed XML to a file
    pub fn write_to_file(&self, file: &mut File) -> Result<(), AxmlError> {
        match self.to_string() {
            Ok(str_xml) => {
                file.write_all(str_xml.as_bytes())?;
                Ok(())
            },
            Err(err) => { Err(err) }
        }
    }

    /// Convert the whole parsed XML into a string
    pub fn to_string(&self) -> Result<String, AxmlError> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;

        self.root.borrow().write_element(&mut writer)?;

        let result = std::str::from_utf8(&writer.into_inner())?
            .to_string();

        Ok(result)
    }

    /// Get a reference to the root of the AXML document
    pub fn root(&self) -> &XmlNode {
        &self.root
    }

    /// Returns a non-consuming iterator over the AXML doc elements
    pub fn iter(&self) -> AxmlIterator {
        AxmlIterator {
            stack: vec![Rc::clone(&self.root)]
        }
    }
}

/// Iterator over an AXML doc
///
/// Iterates through all of the parsed AXML doc elements using depth-first search
pub struct AxmlIterator {
    /// Stack of nodes for depth-first traversal
    stack: Vec<XmlNode>,
}

impl IntoIterator for Axml {
    type Item = XmlNode;
    type IntoIter = AxmlIterator;

    fn into_iter(self) -> Self::IntoIter {
        AxmlIterator {
            stack: vec![Rc::clone(&self.root)],
        }
    }
}

impl Iterator for AxmlIterator {
    type Item = XmlNode;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(node) => {
                for child in &node.borrow().children {
                    self.stack.push(Rc::clone(child));
                }
                Some(node)
            },
            None => None,
        }
    }
}

/// Parse the start of a namepace
pub fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                             strings: &[String],
                             namespaces: &mut HashMap::<String, String>) -> Result<(), AxmlError> {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlStartNamespaceType)?;

    let _line_number = axml_buff.read_u32::<LittleEndian>()?;
    let _comment = axml_buff.read_u32::<LittleEndian>()?;
    let prefix = axml_buff.read_u32::<LittleEndian>()?;
    let uri = axml_buff.read_u32::<LittleEndian>()?;

    let prefix_str = strings.get(prefix as usize).ok_or(AxmlError::StringPoolError)?;
    let uri_str = strings.get(uri as usize).ok_or(AxmlError::StringPoolError)?;
    namespaces.insert(uri_str.to_string(), prefix_str.to_string());

    Ok(())
}

/// Parse the end of a namepace
pub fn parse_end_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                           _strings: &[String]) -> Result<(), AxmlError> {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlEndNamespaceType)?;

    let _line_number = axml_buff.read_u32::<LittleEndian>()?;
    let _comment = axml_buff.read_u32::<LittleEndian>()?;
    let _prefix = axml_buff.read_u32::<LittleEndian>()?;
    let _uri = axml_buff.read_u32::<LittleEndian>()?;

    Ok(())
}

/// Parse the start of an element
pub fn parse_start_element(axml_buff: &mut Cursor<Vec<u8>>,
                           strings: &[String],
                           namespace_prefixes: &HashMap::<String, String>) -> Result<XmlElement, AxmlError> {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlStartElementType)?;

    let _line_number = axml_buff.read_u32::<LittleEndian>()?;
    let _comment = axml_buff.read_u32::<LittleEndian>()?;
    let _namespace = axml_buff.read_u32::<LittleEndian>()?;
    let name = axml_buff.read_u32::<LittleEndian>()?;
    let _attribute_size = axml_buff.read_u32::<LittleEndian>()?;
    let attribute_count = axml_buff.read_u16::<LittleEndian>()?;
    let _id_index = axml_buff.read_u16::<LittleEndian>()?;
    let _class_index = axml_buff.read_u16::<LittleEndian>()?;
    let _style_index = axml_buff.read_u16::<LittleEndian>()?;

    let element_type = strings.get(name as usize).ok_or(AxmlError::StringPoolError)?.to_string();

    let mut decoded_attrs = HashMap::<String, String>::new();
    for _ in 0..attribute_count {
        let attr_namespace = axml_buff.read_u32::<LittleEndian>()?;
        let attr_name = axml_buff.read_u32::<LittleEndian>()?;
        let attr_raw_val = axml_buff.read_u32::<LittleEndian>()?;
        let data_value_type = ResValue::from_buff(axml_buff)?;

        let mut decoded_attr_key = String::new();
        let mut decoded_attr_val = String::new();

        if attr_namespace != 0xffffffff {
            let namespace = strings.get(attr_namespace as usize).ok_or(AxmlError::StringPoolError)?;
            let ns_prefix = namespace_prefixes.get(namespace).ok_or(AxmlError::NamespaceError)?;
            decoded_attr_key.push_str(ns_prefix);
            decoded_attr_key.push(':');
        } else {
            // TODO
        }

        decoded_attr_key.push_str(strings.get(attr_name as usize).ok_or(AxmlError::StringPoolError)?);

        if attr_raw_val != 0xffffffff {
            decoded_attr_val.push_str(&strings.get(attr_raw_val as usize).ok_or(AxmlError::StringPoolError)?.to_string());
        } else {
            match data_value_type.data_type {
                DataValueType::TypeNull => println!("TODO: DataValueType::TypeNull"),
                DataValueType::TypeReference => {
                    decoded_attr_val.push_str("type1/");
                    decoded_attr_val.push_str(&data_value_type.data.to_string());
                },
                DataValueType::TypeAttribute => println!("TODO: DataValueType::TypeAttribute"),
                DataValueType::TypeString => println!("TODO: DataValueType::TypeString"),
                DataValueType::TypeFloat => println!("TODO: DataValueType::TypeFloat"),
                DataValueType::TypeDimension => println!("TODO: DataValueType::TypeDimension"),
                DataValueType::TypeFraction => println!("TODO: DataValueType::TypeFraction"),
                DataValueType::TypeDynamicReference => println!("TODO: DataValueType::TypeDynamicReference"),
                DataValueType::TypeDynamicAttribute => println!("TODO: DataValueType::TypeDynamicAttribute"),
                DataValueType::TypeIntDec => decoded_attr_val.push_str(&data_value_type.data.to_string()),
                DataValueType::TypeIntHex => {
                    decoded_attr_val.push_str("0x");
                    decoded_attr_val.push_str(&format!("{:x}", &data_value_type.data).to_string());
                },
                DataValueType::TypeIntBoolean => {
                    if data_value_type.data == 0 {
                        decoded_attr_val.push_str("false");
                    } else {
                        decoded_attr_val.push_str("true");
                    }
                },
                DataValueType::TypeIntColorArgb8 => println!("TODO: DataValueType::TypeIntColorArgb8"),
                DataValueType::TypeIntColorRgb8 => println!("TODO: DataValueType::TypeIntColorRgb8"),
                DataValueType::TypeIntColorArgb4 => println!("TODO: DataValueType::TypeIntColorArgb4"),
                DataValueType::TypeIntColorRgb4 => println!("TODO: DataValueType::TypeIntColorRgb4"),
            }
        }
        decoded_attrs.insert(
                decoded_attr_key.to_string(),
                decoded_attr_val.to_string()
        );
    }

    Ok(XmlElement {
        element_type,
        attributes: decoded_attrs,
        children: Vec::new()
    })
}

/// Parse the end of an element
pub fn parse_end_element(axml_buff: &mut Cursor<Vec<u8>>,
                         strings: &[String]) -> Result<String, AxmlError> {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlEndElementType)?;

    let _line_number = axml_buff.read_u32::<LittleEndian>()?;
    let _comment = axml_buff.read_u32::<LittleEndian>()?;
    let _namespace = axml_buff.read_u32::<LittleEndian>()?;
    let name = axml_buff.read_u32::<LittleEndian>()?;

    let name = strings.get(name as usize).ok_or(AxmlError::StringPoolError)?;
    Ok(name.to_string())
}

/// Parse a whole XML document
pub fn parse_xml(mut axml_cursor: Cursor<Vec<u8>>) -> Result<Axml, AxmlError> {
    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();

    let root = Rc::new(RefCell::new(XmlElement {
        element_type: "manifest".to_string(),
        attributes: HashMap::new(),
        children: Vec::new()
    }));
    let mut stack = vec![Rc::clone(&root)];
    // let mut stack: Vec<Rc<RefCell<XmlElement>>> = Vec::new();

    while let Ok(block_type) = ChunkType::parse_block_type(&mut axml_cursor) {
        match block_type {
            ChunkType::ResNullType => continue,
            ChunkType::ResStringPoolType => {
                let _ = StringPool::from_buff(&mut axml_cursor, &mut global_strings)?;
            },
            ChunkType::ResTableType => {
                let _ = ResTable::parse(&mut axml_cursor)?;
            },
            ChunkType::ResXmlType => {
                axml_cursor.set_position(axml_cursor.position() - 2);
                let _ = ChunkHeader::from_buff(&mut axml_cursor, ChunkType::ResXmlType)?;
            },
            ChunkType::ResXmlStartNamespaceType => {
                parse_start_namespace(&mut axml_cursor, &global_strings, &mut namespace_prefixes)?;
            },
            ChunkType::ResXmlEndNamespaceType => {
                parse_end_namespace(&mut axml_cursor, &global_strings)?;
            },
            ChunkType::ResXmlStartElementType => {
                let element = parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes)?;

                if element.element_type == "manifest" {
                    stack.last().unwrap().borrow_mut().attributes = element.attributes.clone();
                } else {
                    let new_element = Rc::new(RefCell::new(element));
                    stack.last().unwrap().borrow_mut().children.push(Rc::clone(&new_element));
                    stack.push(new_element);
                }

            },
            ChunkType::ResXmlEndElementType => {
                parse_end_element(&mut axml_cursor, &global_strings)?;
                stack.pop();
            },

            ChunkType::ResXmlResourceMapType => {
                let _ = ResourceMap::from_buff(&mut axml_cursor)?;
            },

            _ => { },
        }
    }

    Ok(Axml { root })
}
