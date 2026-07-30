//! Procedural macros for qlexpress.
//!
//! Currently exposes:
//! - `#[derive(QLExpressType)]` — auto-implement
//!   `qlexpress::runtime::member::QLExpressNativeType` and
//!   `qlexpress::runtime::native_object::NativeObject` for a struct.
//!
//! See README and the design notes in plan.md Stage 6 for the attribute
//! grammar and emitted code shape.

use proc_macro::TokenStream;
use quote::quote;

mod attrs;
mod convert;
mod native_object;
mod native_type;

/// Derive macro for QLExpress-native types.
///
/// # Attributes
///
/// On the struct itself:
/// - `#[qlexpress(name = "com.foo.Bar")]` — canonical Java-style type name
///   (defaults to the Rust struct identifier).
/// - `#[qlexpress(no_native_object)]` — do not emit `impl NativeObject`.
///   Useful when the type is only used through registry closures.
///
/// On fields:
/// - `#[qlexpress(skip)]` — do not expose the field.
/// - `#[qlexpress(readonly)]` — expose only the getter (no setter).
/// - `#[qlexpress(alias = "X", "Y")]` — add field aliases.
///   (The first alias is also registered as a getter method matching the
///   Java `isX/getX` convention.)
///
/// # Limitations (v1)
///
/// - Generic structs / generic methods are rejected with a clear error.
/// - Rust derive 宏看不到独立的 `impl` 块，因此不会自动注册方法或构造器；
///   宿主必须通过 `NativeRegistry` 显式注册这些成员。
///
/// 对应 Java: 无（Rust 原生适配）。
#[proc_macro_derive(QLExpressType, attributes(qlexpress))]
pub fn derive_ql_express_type(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(ast: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let qlexpress_path: syn::Path = syn::parse_quote!(::qlexpress);

    let struct_attrs = attrs::ContainerAttrs::from_ast(ast)?;
    let item = attrs::ItemSpec::from_ast(ast)?;

    let native_type_impl = native_type::generate(ast, &item, &struct_attrs, &qlexpress_path);
    let native_object_impl = native_object::generate(ast, &item, &struct_attrs, &qlexpress_path);

    // 始终生成两个实现。当前 trait 约束要求 `QLExpressNativeType` 同时实现
    // `NativeObject`，因此 `no_native_object` 仍作为后续版本保留字段。
    let _ = struct_attrs.no_native_object;
    Ok(quote! {
        #native_type_impl
        #native_object_impl
    })
}
