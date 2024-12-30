//! Main parser routine
//!
//! This module contains the logic to parse the binary XML into a tree structure (`XmlElement`),
//! representing the actual XML.

use std::collections::HashMap;
use std::borrow::Cow;
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
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::events::attributes::Attribute;
use quick_xml::name::QName;

use crate::chunk_types::ChunkType;
use crate::chunk_header::ChunkHeader;
use crate::data_value_type::DataValueType;
use crate::res_value::ResValue;
use crate::{ ResourceMap, StringPool, ResTable };

/// Representation of an XML element with optional children
#[derive(Debug)]
pub struct XmlElement {
    /// Type of element (e.g., `activity`, `service`)
    pub element_type: String,
    /// Attributes of the element (e.g., `exported`, `permission`)
    pub attributes: HashMap<String, String>,
    /// Vector of children of the XML element
    pub children: Vec<Rc<RefCell<XmlElement>>>,
}

impl XmlElement {
    pub fn write_to_file(&self, file: &mut File) -> Result<(), Error> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .unwrap();

        self.write_element(&mut writer).unwrap();

        file.write_all(&writer.into_inner()[..])
            .expect("Couldn't write to file");

        Ok(())
    }

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
}

/// Parse the start of a namepace
pub fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                             strings: &[String],
                             namespaces: &mut HashMap::<String, String>) {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlStartNamespaceType)
                 .expect("Error: cannot get header from start namespace chunk");

    let _line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let prefix = axml_buff.read_u32::<LittleEndian>().unwrap();
    let uri = axml_buff.read_u32::<LittleEndian>().unwrap();

    let prefix_str = strings.get(prefix as usize).unwrap();
    let uri_str = strings.get(uri as usize).unwrap();
    namespaces.insert(uri_str.to_string(), prefix_str.to_string());
}

/// Parse the end of a namepace
pub fn parse_end_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                           _strings: &[String]) {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlEndNamespaceType)
                 .expect("Error: cannot get header from start namespace chunk");

    let _line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _prefix = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _uri = axml_buff.read_u32::<LittleEndian>().unwrap();
}

/// Parser the start of an element
pub fn parse_start_element(axml_buff: &mut Cursor<Vec<u8>>,
                           strings: &[String],
                           namespace_prefixes: &HashMap::<String, String>) -> XmlElement {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlStartElementType)
                 .expect("Error: cannot get header from start namespace chunk");

    let _line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
    let name = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _attribute_size = axml_buff.read_u32::<LittleEndian>().unwrap();
    let attribute_count = axml_buff.read_u16::<LittleEndian>().unwrap();
    let _id_index = axml_buff.read_u16::<LittleEndian>().unwrap();
    let _class_index = axml_buff.read_u16::<LittleEndian>().unwrap();
    let _style_index = axml_buff.read_u16::<LittleEndian>().unwrap();

    let element_type = strings.get(name as usize).unwrap().to_string();

    let mut decoded_attrs = HashMap::<String, String>::new();
    for _ in 0..attribute_count {
        let attr_namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_name = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_raw_val = axml_buff.read_u32::<LittleEndian>().unwrap();
        let data_value_type = ResValue::from_buff(axml_buff).unwrap();

        let mut decoded_attr_key = String::new();
        let mut decoded_attr_val = String::new();

        if attr_namespace != 0xffffffff {
            let ns_prefix = namespace_prefixes.get(strings.get(attr_namespace as usize).unwrap()).unwrap();
            decoded_attr_key.push_str(ns_prefix);
            decoded_attr_key.push(':');
        } else {
            // TODO
        }

        decoded_attr_key.push_str(strings.get(attr_name as usize).unwrap());

        if attr_raw_val != 0xffffffff {
            decoded_attr_val.push_str(&strings.get(attr_raw_val as usize).unwrap().to_string());
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

    XmlElement {
        element_type,
        attributes: decoded_attrs,
        children: Vec::new()
    }
}

/// Parser the end of an element
pub fn parse_end_element(axml_buff: &mut Cursor<Vec<u8>>,
                         strings: &[String]) -> Result<String, Error> {
    // Go back 2 bytes, to account from the block type
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    // Parse chunk header
    let _header = ChunkHeader::from_buff(axml_buff, ChunkType::ResXmlEndElementType)
                 .expect("Error: cannot get header from start namespace chunk");

    let _line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let _namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
    let name = axml_buff.read_u32::<LittleEndian>().unwrap();

    Ok(strings.get(name as usize).unwrap().to_string())
}

/// Handler for XML events
pub fn handle_event<T> (writer: &mut Writer<T>,
                        element_name: String,
                        element_attrs: Vec<(String, String)>,
                        namespace_prefixes: &HashMap::<String, String>,
                        block_type: ChunkType) where T: std::io::Write {
    match block_type {
        ChunkType::ResXmlStartElementType => {
            // let mut elem = BytesStart::from_content(element_name.as_bytes(), element_name.len());
            let mut elem = BytesStart::new(&element_name);

            if element_name == "manifest" {
                for (k, v) in namespace_prefixes.iter() {
                    if v == "android" {
                        let mut key = String::new();
                        key.push_str("xmlns:");
                        key.push_str(v);
                        let attr = Attribute {
                            key: QName(key.as_bytes()),
                            value: Cow::Borrowed(k.as_bytes())
                        };
                        elem.push_attribute(attr);
                        break;
                    }
                }
            }

            for (attr_key, attr_val) in element_attrs {
                let attr = Attribute {
                    key: QName(attr_key.as_bytes()),
                    value: Cow::Borrowed(attr_val.as_bytes())
                };
                elem.push_attribute(attr);
            }

            assert!(writer.write_event(Event::Start(elem)).is_ok());

        },
        ChunkType::ResXmlEndElementType => {
            assert!(writer.write_event(Event::End(BytesEnd::new(element_name))).is_ok());
        },
        _ => println!("{:02X}, other", block_type),
    }
}

/// Parse a whole XML document
pub fn parse_xml(mut axml_cursor: Cursor<Vec<u8>>) -> Rc<RefCell<XmlElement>> {
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
                let _ = StringPool::from_buff(&mut axml_cursor, &mut global_strings);
            },
            ChunkType::ResTableType => {
                ResTable::parse(&mut axml_cursor);
            },
            ChunkType::ResXmlType => {
                axml_cursor.set_position(axml_cursor.position() - 2);
                let _ = ChunkHeader::from_buff(&mut axml_cursor, ChunkType::ResXmlType);
            },
            ChunkType::ResXmlStartNamespaceType => {
                parse_start_namespace(&mut axml_cursor, &global_strings, &mut namespace_prefixes);
            },
            ChunkType::ResXmlEndNamespaceType => {
                parse_end_namespace(&mut axml_cursor, &global_strings);
            },
            ChunkType::ResXmlStartElementType => {
                // let (element_type, attrs) = parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes).unwrap();
                let element = parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes);

                if element.element_type == "manifest" {
                    stack.last().unwrap().borrow_mut().attributes = element.attributes.clone();
                } else {
                    let new_element = Rc::new(RefCell::new(element));
                    stack.last().unwrap().borrow_mut().children.push(Rc::clone(&new_element));
                    stack.push(new_element);
                }

            },
            ChunkType::ResXmlEndElementType => {
                parse_end_element(&mut axml_cursor, &global_strings).unwrap();
                stack.pop();
            },

            ChunkType::ResXmlResourceMapType => {
                let _ = ResourceMap::from_buff(&mut axml_cursor);
            },

            _ => { },
        }
    }

    root
}
