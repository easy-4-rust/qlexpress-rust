//! Maps a Rust `syn::Type` to the engine's `TargetType` and emits the
//! matching `TargetType` token and value-conversion expression.
//!
//! Only the scalar subset is exhaustive; complex types fall back to
//! `TargetType::Any` and runtime downcasting.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Returns the `TargetType` token that matches the given Rust type, or
/// `None` if the type is unsupported.
pub fn target_type_tokens(ty: &syn::Type) -> Option<TokenStream> {
    let path = last_path_segment(ty)?;
    let ident = &path.ident;
    let name = ident.to_string();
    let variant = match name.as_str() {
        "bool" => quote!(
            ::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Boolean
        ),
        "i8" | "u8" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Byte)
        }
        "i16" | "u16" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Short)
        }
        "i32" | "u32" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Int)
        }
        "i64" | "u64" | "isize" | "usize" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Long)
        }
        "f32" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Float)
        }
        "f64" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Double)
        }
        "i128" | "u128" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::BigInteger)
        }
        "char" => {
            quote!(
                ::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Character
            )
        }
        "String" | "str" => {
            quote!(::qlexpress_rust::runtime::data::convert::obj_type_convertor::TargetType::Any)
        }
        _ => {
            // Custom types fall back to Any (DataValue passthrough).
            // The generated closure will downcast at runtime.
            return None;
        }
    };
    Some(variant)
}

/// True if the type is a numeric scalar that the macro can convert
/// directly via the engine helpers.
pub fn is_supported_scalar(ty: &syn::Type) -> bool {
    matches!(
        scalar_kind(ty),
        Some(ScalarKind::Bool | ScalarKind::Integer | ScalarKind::Float | ScalarKind::String)
    )
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    Bool,
    Integer,
    Float,
    String,
    DataValue,
    Unsupported,
}

pub fn scalar_kind(ty: &syn::Type) -> Option<ScalarKind> {
    let path = last_path_segment(ty)?;
    match path.ident.to_string().as_str() {
        "bool" => Some(ScalarKind::Bool),
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "isize" | "usize"
        | "i128" | "u128" => Some(ScalarKind::Integer),
        "f32" | "f64" => Some(ScalarKind::Float),
        "String" | "str" => Some(ScalarKind::String),
        "DataValue" => Some(ScalarKind::DataValue),
        _ => Some(ScalarKind::Unsupported),
    }
}

/// Returns a token stream that converts a `DataValue` reference into the
/// concrete Rust type. For `DataValue` itself the conversion is identity.
pub fn expr_from_data_value(ty: &syn::Type, val: TokenStream) -> TokenStream {
    let path = match last_path_segment(ty) {
        Some(p) => p,
        None => return quote!(#val),
    };
    let kind = scalar_kind(ty);
    match kind {
        Some(ScalarKind::Bool) => quote!((#val).bool_value_of()),
        Some(ScalarKind::Integer) => {
            let helper = integer_helper(&path.ident.to_string());
            quote!((#val).#helper())
        }
        Some(ScalarKind::Float) => {
            let helper = float_helper(&path.ident.to_string());
            quote!((#val).#helper())
        }
        Some(ScalarKind::String) => {
            // For `String` we clone; for `&str` we cannot easily return a
            // borrowed slice that outlives the call (the engine consumes
            // the converted value), so we always produce an owned `String`.
            // v1 limitation: structs with `&str` fields are not supported.
            quote!((#val).string_value_of())
        }
        Some(ScalarKind::DataValue) => quote!(#val.clone()),
        Some(ScalarKind::Unsupported) | None => quote!(#val.clone()),
    }
}

fn integer_helper(name: &str) -> proc_macro2::Ident {
    // All integer types funnel through `to_i64`; conversion to a smaller
    // width happens via `as` in the caller (the macro wraps the call).
    format_ident!("to_i64")
}

fn float_helper(_name: &str) -> proc_macro2::Ident {
    format_ident!("to_f64")
}

/// Wraps a Rust scalar return value into the corresponding `DataValue`.
pub fn expr_to_data_value(ty: &syn::Type, val: TokenStream) -> TokenStream {
    let path = match last_path_segment(ty) {
        Some(p) => p,
        None => return quote!(::qlexpress_rust::runtime::value::DataValue::Null),
    };
    match scalar_kind(ty).unwrap_or(ScalarKind::Unsupported) {
        ScalarKind::Bool => quote!(::qlexpress_rust::runtime::value::DataValue::Bool(#val)),
        ScalarKind::Integer => {
            let n = path.ident.to_string();
            // Cast to i64 (DataValue::Long stores i64). Narrow types
            // (i8/u8/i16/u16/i32/u32) are first widened to i64 via `as i64`.
            if matches!(
                n.as_str(),
                "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "isize" | "usize"
            ) {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(
                    (#val) as i64
                ))
            } else if n == "u64" {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(
                    (#val).try_into().unwrap_or(0)
                ))
            } else {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(#val))
            }
        }
        ScalarKind::Float => {
            let n = path.ident.to_string();
            if n == "f32" {
                quote!(::qlexpress_rust::runtime::value::DataValue::Double(
                    (#val) as f64
                ))
            } else {
                quote!(::qlexpress_rust::runtime::value::DataValue::Double(#val))
            }
        }
        ScalarKind::String => {
            quote!(::qlexpress_rust::runtime::value::DataValue::Str((#val).to_string()))
        }
        ScalarKind::DataValue => quote!(#val),
        ScalarKind::Unsupported => quote!(::qlexpress_rust::runtime::value::DataValue::Null),
    }
}

fn last_path_segment(ty: &syn::Type) -> Option<&syn::PathSegment> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last(),
        _ => None,
    }
}
