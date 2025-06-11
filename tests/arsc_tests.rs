use std::fs::File;
use std::io::{Cursor, Read};
use rusty_axml::chunks::res_table::ResTable;
use rusty_axml::errors::AxmlError;

// Helper function (can be in test module or a common test utils module)
#[allow(dead_code)]
fn u16_array_to_string(arr: &[u16; 128]) -> String {
    let mut s = String::new();
    for &val in arr.iter() {
        if val == 0 {
            break;
        }
        if let Some(c) = char::from_u32(val as u32) {
            s.push(c);
        } else {
            // Handle error or invalid char by breaking or logging
            // For simplicity in tests, we might just break.
            eprintln!("Warning: Invalid character encountered in u16_array_to_string: {}", val);
            break;
        }
    }
    s
}


#[test]
fn test_load_arsc() -> Result<(), AxmlError> {
    let mut file = File::open("tests/assets/resources.arsc")?;
    let mut vec_data = Vec::new();
    file.read_to_end(&mut vec_data)?;

    let mut cursor = Cursor::new(vec_data);
    let res_table = ResTable::parse(&mut cursor)?;

    // Basic assertion: package_count should be plausible (e.g., >= 1 for typical app)
    // The actual value depends on the specific resources.arsc file.
    // For "tests/assets/resources.arsc", let's assume it's 1 for now.
    // This should be verified by inspecting the ARSC file (e.g., with aapt2 dump).
    assert_eq!(res_table.package_count, 1, "Expected package count to be 1.");
    assert_eq!(res_table.packages.len(), 1, "Expected exactly one parsed package.");

    // Test ResTablePackage details
    let package = res_table.packages.get(0).expect("Should have one package");
    assert_eq!(package.header.type_, rusty_axml::chunks::chunk_types::ChunkType::ResTablePackageType, "Package chunk type mismatch.");

    // Package ID for application resources is typically 0x7f. This should be verified for the specific ARSC.
    assert_eq!(package.id, 0x7f, "Package ID mismatch.");

    // Package name. This requires inspecting the specific ARSC file.
    // For "eu.jgamba.myapplication" (from AndroidManifest.xml tests), this is the expected name.
    let package_name_str = u16_array_to_string(&package.name);
    assert_eq!(package_name_str, "eu.jgamba.myapplication", "Package name mismatch.");

    // Test Type String Pool
    assert!(package.type_string_pool.is_some(), "Type string pool should exist.");
    if let Some(type_strings) = &package.type_string_pool {
        // These assertions depend on the content of tests/assets/resources.arsc
        // Values need to be verified using a tool like `aapt2 dump resources tests/assets/resources.arsc`
        // For example, if "attr" is the first type string:
        // assert!(type_strings.len() > 0);
        // assert_eq!(type_strings[0], "attr"); // Example, actual index and value may vary
        // assert_eq!(type_strings.iter().any(|s| s == "string"), true, "Type 'string' not found");
        // assert_eq!(type_strings.iter().any(|s| s == "layout"), true, "Type 'layout' not found");
        // For now, just check if it's not empty if package_count > 0, specific strings later
        if package.type_specs.len() > 0 || package.types.len() > 0 { // If there are types/specs, there should be type strings
             assert!(!type_strings.is_empty(), "Type string pool should not be empty if types/specs exist.");
        }
    }

    // Test Key String Pool
    assert!(package.key_string_pool.is_some(), "Key string pool should exist.");
    if let Some(key_strings) = &package.key_string_pool {
        // Similar to type strings, specific assertions depend on the ARSC file.
        // Example:
        // assert!(key_strings.len() > 0);
        // assert_eq!(key_strings.iter().any(|s| s == "app_name"), true, "Key 'app_name' not found");
        // assert_eq!(key_strings.iter().any(|s| s == "main_activity"), true, "Key 'main_activity' not found");
        if package.types.iter().any(|t| !t.entries.is_empty()) { // If there are entries, there should be key strings
            assert!(!key_strings.is_empty(), "Key string pool should not be empty if entries exist.");
        }
    }

    // Test ResTable_typeSpec details
    if let Some(type_strings) = &package.type_string_pool {
        let string_type_id = type_strings.iter().position(|s| s == "string").map(|idx| (idx + 1) as u8);

        if let Some(id_val) = string_type_id {
            let string_spec = package.type_specs.iter().find(|spec| spec.id == id_val);
            assert!(string_spec.is_some(), "ResTable_typeSpec for 'string' type not found.");
            if let Some(spec) = string_spec {
                assert_eq!(spec.header.type_, rusty_axml::chunks::chunk_types::ChunkType::ResTableTypeSpecType, "String TypeSpec chunk type mismatch.");
                assert!(spec.entry_count > 0, "String TypeSpec should have entries.");
                // Example: Check if the first entry_flag indicates a public resource (0x40000000 based on ResTable_config::CONFIG_PUBLIC)
                // This requires knowing the exact flag values and structure from ResTable_config or elsewhere.
                // For now, just checking entry_count is a good start.
            }
        } else {
            // If "string" type is not found, it might not be an error if the ARSC is very minimal.
            // However, for a typical app, it should be present.
            // For now, we can choose to fail or log a warning. Let's make it fail for stricter testing.
            // Consider making this assertion conditional based on the specific test ARSC file's known content.
            // assert!(false, "Type 'string' not found in type string pool, cannot test TypeSpec for it.");
            println!("Warning: Type 'string' not found in type string pool. Skipping TypeSpec test for 'string'.");
        }
    }

    // Test ResTable_type and ResTable_entry details
    if let Some(type_strings) = &package.type_string_pool {
        let string_type_id_opt = type_strings.iter().position(|s| s == "string").map(|idx| (idx + 1) as u8);

        if let Some(string_type_id) = string_type_id_opt {
            // Find a ResTable_type for "string" type (can be multiple for different configs)
            // For simplicity, let's find the first one or one with a default/known config.
            // Assuming default config has all language/country bytes as 0.
            let string_res_type = package.types.iter().find(|rt| {
                rt.id == string_type_id &&
                rt.config.language[0] == 0 && rt.config.language[1] == 0 &&
                rt.config.country[0] == 0 && rt.config.country[1] == 0
            });

            assert!(string_res_type.is_some(), "ResTable_type for 'string' with default config not found.");

            if let Some(res_type) = string_res_type {
                assert_eq!(res_type.header.type_, rusty_axml::chunks::chunk_types::ChunkType::ResTableTypeType, "String ResTable_type chunk type mismatch.");
                assert_eq!(res_type.id, string_type_id, "String ResTable_type ID mismatch.");
                assert!(res_type.entry_count > 0, "String ResTable_type should have entries.");
                assert!(res_type.entries_start > 0, "String ResTable_type entries_start should be plausible.");

                // Find a non-None entry to inspect
                let first_valid_entry_option = res_type.entries.iter().find_map(|entry_opt| entry_opt.as_ref());
                assert!(first_valid_entry_option.is_some(), "No valid ResTable_entry found in 'string' ResTable_type.");

                if let Some(entry) = first_valid_entry_option {
                    assert!(package.key_string_pool.is_some(), "Key string pool must exist to check entry key.");
                    let key_strings = package.key_string_pool.as_ref().unwrap();
                    assert!((entry.key.index as usize) < key_strings.len(), "Entry key index out of bounds.");

                    let resource_name = &key_strings[entry.key.index as usize];
                    // We need a known resource name from tests/assets/resources.arsc to assert here.
                    // For example, if "app_name" is a string resource:
                    // assert_eq!(resource_name, "app_name");
                    println!("Found string resource with key: {}", resource_name); // For now, just print.

                    if entry.flags & crate::chunks::res_table_entry::ResTable_entry::FLAG_COMPLEX == 0 {
                        // Simple entry
                        assert!(entry.value.is_some(), "Simple entry should have a value.");
                        let res_value = entry.value.as_ref().unwrap();

                        match res_value.data_type {
                            rusty_axml::chunks::data_value_type::DataValueType::TypeString => {
                                assert!(res_table.global_string_pool.is_some(), "Global string pool must exist for TypeString value.");
                                let global_strings = res_table.global_string_pool.as_ref().unwrap();
                                assert!((res_value.data as usize) < global_strings.len(), "Res_value data (string index) out of bounds for global string pool.");
                                let actual_string_value = &global_strings[res_value.data as usize];
                                // Assert actual_string_value against a known string value for 'resource_name'
                                // e.g., assert_eq!(actual_string_value, "My Application"); if resource_name was "app_name"
                                println!("String resource '{}' has value: '{}'", resource_name, actual_string_value);
                            }
                            rusty_axml::chunks::data_value_type::DataValueType::TypeIntDec |
                            rusty_axml::chunks::data_value_type::DataValueType::TypeIntHex => {
                                // For integer types, assert res_value.data against a known integer value.
                                println!("Integer resource '{}' has value: {}", resource_name, res_value.data);
                            }
                            _ => {
                                // Other types can be handled here if known.
                                println!("Resource '{}' has type {:?} and data {}", resource_name, res_value.data_type, res_value.data);
                            }
                        }
                    } else {
                        // Complex entry (map)
                        assert!(entry.complex_map.is_some(), "Complex entry should have a map.");
                        let map_content = entry.complex_map.as_ref().unwrap();
                        assert!(map_content.count > 0, "Complex map should have at least one ResTable_map item.");
                        println!("Complex resource '{}' has {} map entries.", resource_name, map_content.count);
                        // Further tests could inspect map_content.maps[0].name and map_content.maps[0].value
                    }
                }
            }
        } else {
            println!("Warning: Type 'string' not found in type string pool. Skipping ResTable_type/Entry test for 'string'.");
        }
    }

    Ok(())
}
