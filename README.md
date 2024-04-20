# serde-sated (sane adjacently tagged enum deserialization [with untagged variant])

This crate provides a derive macro to override default serde::Deserialize behavior when deserializing adjacently tagged enum variants with fallback untagged value.

## What is wrong with default Deserialize?

Take a look at following code:
```rust
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
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

#[test]
fn deserialize_json() {
    let missing_field_b_in_complex_variant = json!({
        "resourceType": "Complex",
        "resource": {
            "a": 2000
        }
    });

    let result = serde_json::from_value::<ResourceStruct>(missing_field_b_in_complex_variant).unwrap();
    eprintln!("Resource: {:#?}", result);
}
```

This will print:
```
Resource: Unknown(
    Object {
        "resource": Object {
            "a": Number(2000),
        },
        "resourceType": String("Complex"),
    },
)
```

As you can see, missing field "b" in Complex variant caused serde to default to untagged variant.

## Solution

This may or may not be the desired behavior - this crate allows to change that, instead of defaulting to untagged variant it will return the correct error.

Let's change the ResourceStruct derive attribute to use `serde_sated::deserialize_enum_with_untagged_as_fallback`
```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_sated::deserialize_enum_with_untagged_as_fallback;

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

#[test]
fn deserialize_json() {
    let missing_field_b_in_complex_variant = json!({
        "resourceType": "Complex",
        "resource": {
            "a": 2000
        }
    });

    let result = serde_json::from_value::<ResourceStruct>(missing_field_b_in_complex_variant);
    eprintln!("Resource: {:#?}", result);
}
```

Now the result will be:
```
Resource: Err(
    Error("missing field `b`", line: 0, column: 0),
)
```
