//! This crate provides a derive macro to override default serde::Deserialize behavior when deserializing adjacently tagged enum variants with fallback untagged value.
//!
//! Refer to `deserialize_enum_with_untagged_as_fallback` for details on how to use it

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, LitStr, Type};

#[derive(Debug)]
struct EnumVariant {
    pub ident: String,
    pub content_type: String,
}

fn path_to_ident(path: &syn::Path) -> String {
    if let Some(ident) = path.get_ident() {
        ident.to_string()
    } else {
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<String>>()
            .join("::")
    }
}

fn has_serde_untagged_attribute(attributes: &[Attribute]) -> bool {
    for attribute in attributes {
        if attribute.path().is_ident("serde") {
            let mut is_untagged = false;
            attribute
                .parse_nested_meta(|meta| {
                    // #[serde(untagged))]
                    if meta.path.is_ident("untagged") {
                        is_untagged = true;
                    }

                    Ok(())
                })
                .unwrap();

            if is_untagged {
                return true;
            }
        }
    }
    false
}

fn get_tag_and_content_attributes(attributes: &[Attribute]) -> (String, String) {
    let mut tag_attribute: Option<String> = None;
    let mut content_attribute: Option<String> = None;

    for attr in attributes {
        if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|meta| {
                // #[serde(tag = "resourceTagField"))]
                if meta.path.is_ident("tag") {
                    let lit: LitStr = meta.value()?.parse()?;
                    tag_attribute = Some(lit.value());
                }
                // #[serde(content = "resourceContentField"))]
                else if meta.path.is_ident("content") {
                    let lit: LitStr = meta.value()?.parse()?;
                    content_attribute = Some(lit.value());
                }

                Ok(())
            })
            .unwrap();
        }
    }

    if tag_attribute.is_none() || content_attribute.is_none() {
        panic!("Tag and content attributes must be set, ex. #[serde(tag = \"resourceType\", content = \"resource\")]");
    }

    (tag_attribute.unwrap(), content_attribute.unwrap())
}

fn generate_if_branch(enum_name: &str, variant: &EnumVariant) -> String {
    format!(
        r#"
        if resource_type == "{0}" {{
            let resource = {1}::deserialize(resource.to_owned())
                .map_err(|e| serde::de::Error::custom(e))?;
            Ok({enum_name}::{0}(resource))
        }}
"#,
        variant.ident, variant.content_type
    )
}

// TODO: support other untagged types than serde_json::Value
fn generate_else_branch(enum_name: &str, variant: &EnumVariant) -> String {
    format!(
        r#"       else {{
            Ok({enum_name}::{}(resource.to_owned()))
        }}
    "#,
        variant.ident
    )
}

fn generate_if_else_tree(
    enum_name: &str,
    variants: &[EnumVariant],
    untagged_variant: &EnumVariant,
) -> String {
    let if_tree = variants
        .iter()
        .map(|variant| generate_if_branch(enum_name, variant))
        .collect::<Vec<_>>()
        .join("else ");
    let else_branch = generate_else_branch(enum_name, untagged_variant);
    format!("{if_tree} {else_branch}")
}

/// deserialize enum adjacently tagged enum without defaulting to untagged variant on failure
///
/// Example:
/// ```
/// use serde::{Deserialize, Serialize};
/// use serde_json::json;
/// use serde_sated::deserialize_enum_with_untagged_as_fallback;
///
/// #[derive(Debug, deserialize_enum_with_untagged_as_fallback, Serialize)]
/// #[serde(tag = "resourceType", content = "resource")]
/// pub enum ResourceStruct {
///     Number(u64),
///     String(String),
///     Complex(Complex),
///     #[serde(untagged)]
///     Unknown(serde_json::Value),
/// }
///
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Complex {
///     pub a: u64,
///     pub b: u64,
/// }
///
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(tag = "resourceType", content = "resource")]
/// pub enum ResourceStructButUsingDefaultDeserialize {
///     Number(u64),
///     String(String),
///     Complex(Complex),
///     #[serde(untagged)]
///     Unknown(serde_json::Value),
/// }
///
/// fn main() {
///     let missing_field_b_in_complex_variant = json!({
///         "resourceType": "Complex",
///         "resource": {
///             "a": 2000
///         }
///     });
///
///     let result = serde_json::from_value::<ResourceStruct>(missing_field_b_in_complex_variant.to_owned());
///     eprintln!("Resource: {:#?}", result);
///     /*
///      * Prints:
///      * Resource: Err(
///      *     Error("missing field `b`", line: 0, column: 0),
///      * )
///      */
///    
///     let result = serde_json::from_value::<ResourceStructButUsingDefaultDeserialize>(missing_field_b_in_complex_variant.to_owned());
///     eprintln!("Resource: {:#?}", result);
///     /*
///      * Prints:
///      *  Resource: Ok(
///      *      Unknown(
///      *          Object {
///      *              "resource": Object {
///      *                  "a": Number(2000),
///      *              },
///      *              "resourceType": String("Complex"),
///      *          },
///      *      ),
///      *  )
///      */
/// }
/// ```
#[proc_macro_derive(deserialize_enum_with_untagged_as_fallback)]
pub fn derive_enum(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let (tag_attribute, content_attribute) = get_tag_and_content_attributes(&input.attrs);

    let enum_name = input.ident.to_string();

    let enum_data = match input.data {
        Data::Struct(_) => panic!("Unsupported type `struct`, must be `enum`"),
        Data::Union(_) => panic!("Unsupported type `union`, must be `enum`"),
        Data::Enum(value) => value,
    };

    let mut variants: Vec<EnumVariant> = vec![];
    let mut untagged_variant: Option<EnumVariant> = None;

    if enum_data.variants.is_empty() {
        panic!("Enum variants are empty");
    }

    for variant in &enum_data.variants {
        let variant_name = variant.ident.to_owned();
        let mut variant_inner_type: Option<String> = None;

        let is_untagged = has_serde_untagged_attribute(&variant.attrs);

        for field in &variant.fields {
            let field_path = match &field.ty {
                Type::Path(field_path) => field_path,
                _ => continue,
            };
            variant_inner_type = Some(path_to_ident(&field_path.path));
        }

        match variant_inner_type {
            Some(value) => {
                let variant = EnumVariant {
                    ident: variant_name.to_string(),
                    content_type: value,
                };

                if is_untagged {
                    untagged_variant = Some(variant);
                } else {
                    variants.push(variant);
                }
            }
            None => panic!("Unable to resolve inner type of {variant_name}"),
        }
    }

    if untagged_variant.is_none() {
        panic!("No untagged variant specified, use serde::Deserialize instead");
    }
    let if_else_tree = generate_if_else_tree(&enum_name, &variants, &untagged_variant.unwrap());

    let output = format!(
        r#"
impl<'de> serde::Deserialize<'de> for {enum_name} {{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {{
        let value = serde_json::Value::deserialize(deserializer)?;
        
        let resource_type = value
            .get("{tag_attribute}")
            .ok_or(serde::de::Error::custom("missing field `{tag_attribute}`"))?
            .as_str()
            .ok_or(serde::de::Error::custom("`{tag_attribute}` is not of type `string`"))?;
            
        let resource = value
            .get("{content_attribute}")
            .ok_or(serde::de::Error::custom("missing field `{content_attribute}`"))?;

        {if_else_tree}
    }}
}}"#
    );

    output.parse().unwrap()
}
