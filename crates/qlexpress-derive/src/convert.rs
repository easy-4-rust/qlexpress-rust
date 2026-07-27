//! 把 Rust 字段值转换为 QLExpress 栈值。
//!
//! 该模块只服务于 `QLExpressType` 派生宏生成的字段 getter。方法参数转换
//! 由宿主显式注册的 `NativeRegistry` 闭包负责。

use proc_macro2::TokenStream;
use quote::quote;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Rust 字段值转换为 `DataValue` 时采用的代码生成策略。
///
/// 该枚举对具体 Rust 类型进行归类，例如所有整数类型归为 `Integer`；
/// 它不是 QLExpress 的运行时类型模型。
enum FieldConversionKind {
    Bool,
    Integer,
    Float,
    String,
    DataValue,
    Unsupported,
}

fn field_conversion_kind(ty: &syn::Type) -> Option<FieldConversionKind> {
    let path = last_path_segment(ty)?;
    match path.ident.to_string().as_str() {
        "bool" => Some(FieldConversionKind::Bool),
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "isize" | "usize"
        | "i128" | "u128" => Some(FieldConversionKind::Integer),
        "f32" | "f64" => Some(FieldConversionKind::Float),
        "String" | "str" => Some(FieldConversionKind::String),
        "DataValue" => Some(FieldConversionKind::DataValue),
        _ => Some(FieldConversionKind::Unsupported),
    }
}

/// 生成把 Rust 字段值包装为 `DataValue` 的表达式。
pub fn expr_to_data_value(ty: &syn::Type, val: TokenStream) -> TokenStream {
    let path = match last_path_segment(ty) {
        Some(path) => path,
        None => return quote!(::qlexpress::runtime::value::DataValue::Null),
    };
    match field_conversion_kind(ty).unwrap_or(FieldConversionKind::Unsupported) {
        FieldConversionKind::Bool => {
            quote!(::qlexpress::runtime::value::DataValue::Bool(#val))
        }
        FieldConversionKind::Integer => {
            let name = path.ident.to_string();
            if matches!(
                name.as_str(),
                "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "isize" | "usize"
            ) {
                quote!(::qlexpress::runtime::value::DataValue::Long(
                    (#val) as i64
                ))
            } else if name == "u64" {
                quote!(::qlexpress::runtime::value::DataValue::Long(
                    (#val).try_into().unwrap_or(0)
                ))
            } else {
                quote!(::qlexpress::runtime::value::DataValue::Long(#val))
            }
        }
        FieldConversionKind::Float => {
            if path.ident == "f32" {
                quote!(::qlexpress::runtime::value::DataValue::Double(
                    (#val) as f64
                ))
            } else {
                quote!(::qlexpress::runtime::value::DataValue::Double(#val))
            }
        }
        FieldConversionKind::String => {
            quote!(::qlexpress::runtime::value::DataValue::Str((#val).to_string()))
        }
        FieldConversionKind::DataValue => quote!(#val),
        FieldConversionKind::Unsupported => {
            quote!(::qlexpress::runtime::value::DataValue::Null)
        }
    }
}

fn last_path_segment(ty: &syn::Type) -> Option<&syn::PathSegment> {
    match ty {
        syn::Type::Path(path) => path.path.segments.last(),
        _ => None,
    }
}
