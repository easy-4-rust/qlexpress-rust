//! Procedural macros for qlexpress_rust.
//!
//! Currently exposes:
//! - `#[derive(QLExpressType)]` — auto-implement
//!   `qlexpress_rust::runtime::member::QLExpressNativeType` and
//!   `qlexpress_rust::runtime::native_object::NativeObject` for a struct.
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
/// On inherent methods:
/// - `#[qlexpress(constructor)]` — designate the function as the type's
///   script-visible constructor.
/// - `#[qlexpress(static)]` — register as a static method (no bean arg).
/// - `#[qlexpress(skip)]` — do not expose.
/// - `#[qlexpress(rename = "other")]` — register under a different name.
/// - `#[qlexpress(alias = "...")]` — register additional aliases.
///
/// # Limitations (v1)
///
/// - Generic structs / generic methods are rejected with a clear error.
/// - `async fn` is rejected.
/// - Methods taking `self` by value are rejected.
/// - Variadic methods are not auto-generated; users must hand-write closures.
/// - One closure per method name (no overloading); use `rename` to disambiguate.
#[proc_macro_derive(QLExpressType, attributes(qlexpress))]
pub fn derive_ql_express_type(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(ast: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let qlexpress_path: syn::Path = syn::parse_quote!(::qlexpress_rust);

    let struct_attrs = attrs::ContainerAttrs::from_ast(ast)?;
    let item = attrs::ItemSpec::from_ast(ast)?;

    let native_type_impl = native_type::generate(ast, &item, &struct_attrs, &qlexpress_path);
    let native_object_impl = native_object::generate(ast, &item, &struct_attrs, &qlexpress_path);

    // Always emit both impls. `no_native_object` is currently a no-op
    // reserved for future expansion; v1 requires the derive to provide
    // `NativeObject` so that `QLExpressNativeType: NativeObject` holds.
    let _ = struct_attrs.no_native_object;
    Ok(quote! {
        #native_type_impl
        #native_object_impl
    })
}
