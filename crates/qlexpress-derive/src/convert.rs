//! 把 Rust 字段值转换为 QLExpress 栈值。
//!
//! 该模块只服务于 `QLExpressType` 派生宏生成的字段 getter。方法参数转换
//! 由宿主显式注册的 `NativeRegistry` 闭包负责。

use proc_macro2::TokenStream;
use quote::quote;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ScalarKind {
    Bool,
    Integer,
    Float,
    String,
    DataValue,
    Unsupported,
}

fn scalar_kind(ty: &syn::Type) -> Option<ScalarKind> {
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

/// 生成把 Rust 字段值包装为 `DataValue` 的表达式。
pub fn expr_to_data_value(ty: &syn::Type, val: TokenStream) -> TokenStream {
    let path = match last_path_segment(ty) {
        Some(path) => path,
        None => return quote!(::qlexpress_rust::runtime::value::DataValue::Null),
    };
    match scalar_kind(ty).unwrap_or(ScalarKind::Unsupported) {
        ScalarKind::Bool => quote!(::qlexpress_rust::runtime::value::DataValue::Bool(#val)),
        ScalarKind::Integer => {
            let name = path.ident.to_string();
            if matches!(
                name.as_str(),
                "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "isize" | "usize"
            ) {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(
                    (#val) as i64
                ))
            } else if name == "u64" {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(
                    (#val).try_into().unwrap_or(0)
                ))
            } else {
                quote!(::qlexpress_rust::runtime::value::DataValue::Long(#val))
            }
        }
        ScalarKind::Float => {
            if path.ident == "f32" {
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
        syn::Type::Path(path) => path.path.segments.last(),
        _ => None,
    }
}
