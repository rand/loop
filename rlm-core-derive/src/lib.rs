//! Derive macros for rlm-core typed signatures.
//!
//! This crate provides the `#[derive(Signature)]` macro for automatically
//! implementing the `Signature` trait on structs.
//!
//! # Example
//!
//! ```ignore
//! use rlm_core::Signature;
//!
//! #[derive(Signature)]
//! #[signature(instructions = "Summarize the given text")]
//! struct Summarize {
//!     #[input(desc = "Text to summarize")]
//!     text: String,
//!
//!     #[input(desc = "Maximum length", prefix = "Max Length")]
//!     max_length: Option<u32>,
//!
//!     #[output(desc = "The summary")]
//!     summary: String,
//!
//!     #[output(desc = "Key points extracted")]
//!     key_points: Vec<String>,
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, Ident, Type,
    Error, spanned::Spanned, LitStr, LitBool,
};

/// Derive macro for implementing the `Signature` trait.
///
/// # Attributes
///
/// ## Struct-level
///
/// - `#[signature(instructions = "...")]` - Required. Sets the task instructions.
///
/// ## Field-level
///
/// - `#[input(desc = "...")]` - Mark field as input with description.
/// - `#[input(desc = "...", prefix = "...")]` - Input with custom display prefix.
/// - `#[output(desc = "...")]` - Mark field as output with description.
/// - `#[output(desc = "...", prefix = "...")]` - Output with custom display prefix.
/// - `#[field(required = false)]` - Mark field as optional (also inferred from `Option<T>`).
/// - `#[field(default = "...")]` - Set default value (JSON).
/// - `#[field(enum_values = "a,b,c")]` - Treat field as enum with explicit allowed values.
///
/// # Generated Code
///
/// The macro generates:
/// - `{Name}Inputs` struct with all `#[input]` fields
/// - `{Name}Outputs` struct with all `#[output]` fields
/// - `Signature` trait implementation
#[proc_macro_derive(Signature, attributes(signature, input, output, field))]
pub fn derive_signature(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_signature_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_signature_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let vis = &input.vis;

    // Parse struct-level attributes
    let signature_attrs = parse_signature_attrs(&input)?;
    let instructions = signature_attrs.instructions.ok_or_else(|| {
        Error::new(
            input.ident.span(),
            "Missing #[signature(instructions = \"...\")] attribute"
        )
    })?;

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(Error::new(
                input.ident.span(),
                "Signature can only be derived for structs with named fields"
            )),
        },
        _ => return Err(Error::new(
            input.ident.span(),
            "Signature can only be derived for structs"
        )),
    };

    // Parse field attributes and separate inputs/outputs
    let mut input_fields = Vec::new();
    let mut output_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let field_attrs = parse_field_attrs(field)?;

        match field_attrs.kind {
            Some(FieldKind::Input) => {
                input_fields.push(ParsedField {
                    name: field_name.clone(),
                    ty: field_type.clone(),
                    attrs: field_attrs,
                });
            }
            Some(FieldKind::Output) => {
                output_fields.push(ParsedField {
                    name: field_name.clone(),
                    ty: field_type.clone(),
                    attrs: field_attrs,
                });
            }
            None => {
                return Err(Error::new(
                    field_name.span(),
                    format!(
                        "Field '{}' must be marked with #[input(...)] or #[output(...)]",
                        field_name
                    )
                ));
            }
        }
    }

    // Validate we have at least one input and output
    if input_fields.is_empty() {
        return Err(Error::new(
            name.span(),
            "Signature must have at least one #[input] field"
        ));
    }
    if output_fields.is_empty() {
        return Err(Error::new(
            name.span(),
            "Signature must have at least one #[output] field"
        ));
    }

    // Generate struct names
    let inputs_name = format_ident!("{}Inputs", name);
    let outputs_name = format_ident!("{}Outputs", name);

    // Generate input struct fields
    let input_struct_fields: Vec<_> = input_fields.iter().map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        quote! { pub #name: #ty }
    }).collect();

    // Generate output struct fields
    let output_struct_fields: Vec<_> = output_fields.iter().map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        quote! { pub #name: #ty }
    }).collect();

    // Generate input field specs
    let input_field_specs: Vec<_> = input_fields.iter().map(|f| {
        generate_field_spec(f)
    }).collect();

    // Generate output field specs
    let output_field_specs: Vec<_> = output_fields.iter().map(|f| {
        generate_field_spec(f)
    }).collect();

    // Generate the implementation
    let expanded = quote! {
        /// Input type for the #name signature.
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        #vis struct #inputs_name {
            #(#input_struct_fields),*
        }

        /// Output type for the #name signature.
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        #vis struct #outputs_name {
            #(#output_struct_fields),*
        }

        impl ::rlm_core::signature::Signature for #name {
            type Inputs = #inputs_name;
            type Outputs = #outputs_name;

            fn instructions() -> &'static str {
                #instructions
            }

            fn input_fields() -> Vec<::rlm_core::signature::FieldSpec> {
                vec![
                    #(#input_field_specs),*
                ]
            }

            fn output_fields() -> Vec<::rlm_core::signature::FieldSpec> {
                vec![
                    #(#output_field_specs),*
                ]
            }
        }
    };

    Ok(expanded)
}

/// Parsed struct-level signature attributes.
#[derive(Default)]
struct SignatureAttrs {
    instructions: Option<String>,
}

/// Parse #[signature(...)] attributes.
fn parse_signature_attrs(input: &DeriveInput) -> Result<SignatureAttrs, Error> {
    let mut result = SignatureAttrs::default();

    for attr in &input.attrs {
        if !attr.path().is_ident("signature") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("instructions") {
                let value: LitStr = meta.value()?.parse()?;
                result.instructions = Some(value.value());
                Ok(())
            } else {
                Err(meta.error("unknown signature attribute"))
            }
        })?;
    }

    Ok(result)
}

/// Kind of field (input or output).
#[derive(Clone, Copy)]
enum FieldKind {
    Input,
    Output,
}

/// Parsed field attributes.
#[derive(Default)]
struct FieldAttrs {
    kind: Option<FieldKind>,
    desc: Option<String>,
    prefix: Option<String>,
    required: Option<bool>,
    default: Option<String>,
    enum_values: Option<Vec<String>>,
}

/// Parse field attributes (#[input], #[output], #[field]).
fn parse_field_attrs(field: &syn::Field) -> Result<FieldAttrs, Error> {
    let mut result = FieldAttrs::default();
    let field_name = field.ident.as_ref().unwrap();

    for attr in &field.attrs {
        if attr.path().is_ident("input") {
            if result.kind.is_some() {
                return Err(Error::new(
                    attr.path().span(),
                    format!("Field '{}' cannot have both #[input] and #[output]", field_name)
                ));
            }
            result.kind = Some(FieldKind::Input);
            parse_io_attr(attr, &mut result)?;
        } else if attr.path().is_ident("output") {
            if result.kind.is_some() {
                return Err(Error::new(
                    attr.path().span(),
                    format!("Field '{}' cannot have both #[input] and #[output]", field_name)
                ));
            }
            result.kind = Some(FieldKind::Output);
            parse_io_attr(attr, &mut result)?;
        } else if attr.path().is_ident("field") {
            parse_field_attr(attr, &mut result)?;
        }
    }

    Ok(result)
}

/// Parse #[input(...)] or #[output(...)] attribute.
fn parse_io_attr(attr: &syn::Attribute, result: &mut FieldAttrs) -> Result<(), Error> {
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("desc") {
            let value: LitStr = meta.value()?.parse()?;
            result.desc = Some(value.value());
            Ok(())
        } else if meta.path.is_ident("prefix") {
            let value: LitStr = meta.value()?.parse()?;
            result.prefix = Some(value.value());
            Ok(())
        } else {
            Err(meta.error("unknown attribute, expected 'desc' or 'prefix'"))
        }
    })
}

/// Parse #[field(...)] attribute.
fn parse_field_attr(attr: &syn::Attribute, result: &mut FieldAttrs) -> Result<(), Error> {
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("required") {
            let value: LitBool = meta.value()?.parse()?;
            result.required = Some(value.value());
            Ok(())
        } else if meta.path.is_ident("default") {
            let value: LitStr = meta.value()?.parse()?;
            result.default = Some(value.value());
            Ok(())
        } else if meta.path.is_ident("enum_values") {
            let value: LitStr = meta.value()?.parse()?;
            let parsed = value
                .value()
                .split(',')
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
                .collect::<Vec<_>>();
            if parsed.is_empty() {
                return Err(meta.error("enum_values cannot be empty"));
            }
            result.enum_values = Some(parsed);
            Ok(())
        } else {
            Err(meta.error("unknown field attribute, expected 'required', 'default', or 'enum_values'"))
        }
    })
}

/// A parsed field with its attributes.
struct ParsedField {
    name: Ident,
    ty: Type,
    attrs: FieldAttrs,
}

/// Generate FieldSpec construction code for a field.
fn generate_field_spec(field: &ParsedField) -> TokenStream2 {
    let name_str = field.name.to_string();
    let field_type = if let Some(values) = &field.attrs.enum_values {
        let value_literals: Vec<_> = values
            .iter()
            .map(|value| LitStr::new(value, field.name.span()))
            .collect();
        quote! {
            ::rlm_core::signature::FieldType::Enum(vec![
                #(::std::string::String::from(#value_literals)),*
            ])
        }
    } else {
        infer_field_type(&field.ty)
    };

    let desc = field.attrs.desc.as_deref().unwrap_or("");

    // Check if type is Option<T> for required inference
    let is_option = is_option_type(&field.ty);
    let required = field.attrs.required.unwrap_or(!is_option);

    let mut builder = quote! {
        ::rlm_core::signature::FieldSpec::new(#name_str, #field_type)
            .with_description(#desc)
    };

    if let Some(prefix) = &field.attrs.prefix {
        builder = quote! { #builder.with_prefix(#prefix) };
    }

    if !required {
        builder = quote! { #builder.optional() };
    }

    if let Some(default) = &field.attrs.default {
        builder = quote! {
            #builder.with_default(::serde_json::json!(#default))
        };
    }

    builder
}

/// Infer FieldType from a Rust type.
fn infer_field_type(ty: &Type) -> TokenStream2 {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;

            // Get the last segment (type name)
            if let Some(segment) = path.segments.last() {
                let ident = &segment.ident;
                let ident_str = ident.to_string();

                match ident_str.as_str() {
                    // String types
                    "String" | "str" => {
                        quote! { ::rlm_core::signature::FieldType::String }
                    }
                    // Integer types
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                        quote! { ::rlm_core::signature::FieldType::Integer }
                    }
                    // Float types
                    "f32" | "f64" => {
                        quote! { ::rlm_core::signature::FieldType::Float }
                    }
                    // Boolean
                    "bool" => {
                        quote! { ::rlm_core::signature::FieldType::Boolean }
                    }
                    // Vec<T> -> List
                    "Vec" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner = infer_field_type(inner_ty);
                                return quote! {
                                    ::rlm_core::signature::FieldType::List(Box::new(#inner))
                                };
                            }
                        }
                        quote! { ::rlm_core::signature::FieldType::List(Box::new(::rlm_core::signature::FieldType::String)) }
                    }
                    // Option<T> -> same as T
                    "Option" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                return infer_field_type(inner_ty);
                            }
                        }
                        quote! { ::rlm_core::signature::FieldType::String }
                    }
                    // Custom type
                    _ => {
                        quote! { ::rlm_core::signature::FieldType::Custom(#ident_str.to_string()) }
                    }
                }
            } else {
                quote! { ::rlm_core::signature::FieldType::Custom("unknown".to_string()) }
            }
        }
        _ => {
            quote! { ::rlm_core::signature::FieldType::Custom("unknown".to_string()) }
        }
    }
}

/// Check if a type is Option<T>.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}
