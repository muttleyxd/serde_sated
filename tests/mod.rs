use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use serde_sated::deserialize_enum_with_untagged_as_fallback;

// // TODO: add support for renaming all fields
#[derive(Debug, deserialize_enum_with_untagged_as_fallback, Serialize)]
#[serde(tag = "resourceType", content = "resource")]
pub enum ResourceStruct {
    Number(u64),
    String(String),
    Complex(Complex),
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Complex {
    pub a: u64,
    pub b: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "resourceType", content = "resource")]
pub enum ResourceStructButUsingDefaultDeserialize {
    Number(u64),
    String(String),
    Complex(Complex),
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

#[test]
fn test_deserialize_json() {
    let missing_field_b_in_complex_variant = json!({
        "resourceType": "Complex",
        "resource": {
            "a": 2000
        }
    });

    let result =
        serde_json::from_value::<ResourceStruct>(missing_field_b_in_complex_variant.to_owned());
    eprintln!("Resource: {:#?}", result);

    let result = serde_json::from_value::<ResourceStructButUsingDefaultDeserialize>(
        missing_field_b_in_complex_variant.to_owned(),
    );
    eprintln!("Resource: {:#?}", result);
}

#[test]
fn test_what_is_wrong_with_default_deserialize() {
    let missing_field_b_in_complex_variant = json!({
        "unrelated": 1234,
        "resourceType": "Complex",
        "resource": {
            "a": 2000
        }
    });

    let result: ResourceStructButUsingDefaultDeserialize =
        serde_json::from_value(missing_field_b_in_complex_variant).unwrap();
    assert!(matches!(
        result,
        ResourceStructButUsingDefaultDeserialize::Unknown(_)
    ));
}

#[test]
fn test_unsuccessful_deserialization_returns_an_error_instead_of_implicitly_switching_to_untagged_variant(
) {
    let missing_field_b_in_complex_variant = json!({
        "resourceType": "Complex",
        "resource": {
            "a": 2000
        }
    });
    let result: Result<ResourceStruct, serde_json::Error> =
        serde_json::from_value(missing_field_b_in_complex_variant);
    let error = result.unwrap_err();
    assert!(format!("{error}").contains("missing field"));
}

#[test]
fn test_successful_deserialization() {
    let variant_string = json!({
        "resourceType": "String",
        "resource": "text"
    });
    let result: ResourceStruct = serde_json::from_value(variant_string).unwrap();
    assert!(matches!(result, ResourceStruct::String(_)));

    let variant_number = json!({
        "unrelated": 1234,
        "resourceType": "Number",
        "resource": 2000
    });
    let result: ResourceStruct = serde_json::from_value(variant_number).unwrap();
    assert!(matches!(result, ResourceStruct::Number(2000)));

    let variant_complex = json!({
        "resourceType": "Complex",
        "resource": {
            "a": 2000,
            "b": 3000,
        }
    });
    let result: ResourceStruct = serde_json::from_value(variant_complex).unwrap();
    assert!(matches!(
        result,
        ResourceStruct::Complex(Complex { a: 2000, b: 3000 })
    ));

    let variant_unknown_but_matching_enum = json!({
        "unrelated": 1234,
        "resourceType": "Unknown",
        "resource": {
            "c": 4000
        }
    });
    let result: ResourceStruct = serde_json::from_value(variant_unknown_but_matching_enum).unwrap();
    assert!(matches!(result, ResourceStruct::Unknown(_)));

    let variant_unknown_matched_by_untagged_type = json!({
        "unrelated": 1234,
        "resourceType": "NEWRANDOMTYPE",
        "resource": {
            "d": 5000
        }
    });
    let result: ResourceStruct =
        serde_json::from_value(variant_unknown_matched_by_untagged_type).unwrap();
    assert!(matches!(result, ResourceStruct::Unknown(_)));
}

#[derive(Debug, deserialize_enum_with_untagged_as_fallback, Serialize)]
#[serde(tag = "resourceType", content = "resource")]
pub enum ResourceStructWithRename {
    #[serde(rename = "string")]
    String(String),
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

#[test]
fn test_rename() {
    let variant_string = json!({
        "resourceType": "string",
        "resource": "text"
    });
    let result: ResourceStructWithRename = serde_json::from_value(variant_string).unwrap();
    assert!(matches!(result, ResourceStructWithRename::String(_)));
}

#[derive(Debug, deserialize_enum_with_untagged_as_fallback, Serialize)]
#[serde(tag = "resourceType", content = "resource")]
pub enum ResourceStructWithDeserializeWith {
    #[serde(deserialize_with = "always_returns_five")]
    Number(u32),
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

fn always_returns_five<'de, D>(_deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(5u32)
}

#[test]
fn test_deserialize_with() {
    let variant_string = json!({
        "resourceType": "Number",
        "resource": 1
    });
    let result: ResourceStructWithDeserializeWith = serde_json::from_value(variant_string).unwrap();
    assert!(matches!(
        result,
        ResourceStructWithDeserializeWith::Number(5)
    ));
}
