//! Generate `impl NativeObject for T`.
//!
//! Field getters live in two places:
//! - `impl NativeObject for T` (this file) — handles calls routed through
//!   `Rc<RefCell<dyn NativeObject>>`. The trait knows `Self`, so we can
//!   `self.as_any().downcast_ref::<Self>()` and read fields directly.
//! - `NativeType.fields` (in `native_type.rs`) — handles calls routed
//!   through the `NativeRegistry`. There `Self` is also known (the
//!   registry caller built the closures from `T::build_native_type`),
//!   so the closure body reuses the same downcast trick.

use proc_macro2::TokenStream;
use quote::quote;

use crate::attrs::{ContainerAttrs, FieldSpec, ItemSpec};
/// 根据派生规格生成对应的 Rust TokenStream。
/// 参数：`_ast`、`item`、`container`、`qlexpress_path`；返回：`TokenStream`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `generate`；Rust 侧按所有权与 `Result` 语义适配。
/// 对应 Java: 无（Rust 原生适配）。
pub fn generate(
    _ast: &syn::DeriveInput,
    item: &ItemSpec,
    container: &ContainerAttrs,
    qlexpress_path: &syn::Path,
) -> TokenStream {
    let ident = &item.ident;
    let type_name_str = container
        .name
        .clone()
        .unwrap_or_else(|| item.ident.to_string());

    let getter_arms: Vec<TokenStream> = item
        .fields
        .iter()
        .filter(|f| !f.attrs.skip)
        .map(gen_getter_arm)
        .collect();
    let mut getter_combined = TokenStream::new();
    for arm in getter_arms {
        getter_combined.extend(arm);
    }

    let alias_lookup = build_alias_lookup(item);
    let setter_arms: Vec<TokenStream> = item
        .fields
        .iter()
        .filter(|f| !f.attrs.skip && !f.attrs.readonly)
        .map(|field| gen_setter_arm(field, qlexpress_path))
        .collect();
    let mut setter_combined = TokenStream::new();
    for arm in setter_arms {
        setter_combined.extend(arm);
    }

    quote! {
        impl #qlexpress_path::runtime::native_object::NativeObject for #ident {
            fn get_field(&self, name: &str) -> Option< #qlexpress_path::runtime::value::DataValue> {
                // Downcast from `&dyn NativeObject` back to `&Self` so
                // we can read struct fields by name. This is the
                // canonical idiom: the `Self` type is known here.
                let me: &Self = match self.as_any().downcast_ref::<Self>() {
                    Some(v) => v,
                    None => return None,
                };
                let canonical = #alias_lookup.get(name).copied().unwrap_or(name);
                match canonical {
                    #getter_combined
                    _ => None,
                }
            }

            fn set_field(
                &mut self,
                name: &str,
                value: &#qlexpress_path::runtime::value::DataValue,
            ) -> bool {
                let canonical = #alias_lookup.get(name).copied().unwrap_or(name);
                match canonical {
                    #setter_combined
                    _ => false,
                }
            }

            fn call_method(
                &mut self,
                _name: &str,
                _args: &[ #qlexpress_path::runtime::value::DataValue],
            ) -> Result< #qlexpress_path::runtime::value::DataValue, #qlexpress_path::exception::QLException> {
                // v1: methods are dispatched via the registry path; the
                // direct `DataValue::Object` cell reports "not found".
                Err(#qlexpress_path::exception::QLException::for_test(
                    #qlexpress_path::exception::ql_exception::QLExceptionKind::Runtime,
                    format!("method not found: {}", _name),
                    #qlexpress_path::exception::error_codes::METHOD_NOT_FOUND,
                ))
            }

            fn native_type_name(&self) -> &str {
                #type_name_str
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        }
    }
}

fn gen_setter_arm(f: &FieldSpec, qlexpress_path: &syn::Path) -> TokenStream {
    let name = f.ident.to_string();
    let field_ident = &f.ident;
    let assignment = crate::convert::assign_data_value_to_field(
        &f.ty,
        quote!(self.#field_ident),
        quote!(value),
        qlexpress_path,
    );
    quote! {
        #name => { #assignment },
    }
}

fn gen_getter_arm(f: &FieldSpec) -> TokenStream {
    let name = f.ident.to_string();
    let field_ident = &f.ident;
    let value_expr = crate::convert::expr_to_data_value(&f.ty, quote!(me.#field_ident));
    quote! {
        #name => Some({ #value_expr }),
    }
}

fn build_alias_lookup(item: &ItemSpec) -> TokenStream {
    let mut inserts: Vec<TokenStream> = Vec::new();
    for f in &item.fields {
        if f.attrs.skip || f.attrs.aliases.is_empty() {
            continue;
        }
        let canonical = f.ident.to_string();
        for alias in &f.attrs.aliases {
            inserts.push(quote! { m.insert(#alias, #canonical); });
        }
    }
    if inserts.is_empty() {
        return quote! { ::std::collections::HashMap::<&str, &str>::new() };
    }
    quote! {
        {
            let mut m: ::std::collections::HashMap<&str, &str> = ::std::collections::HashMap::new();
            #( #inserts )*
            m
        }
    }
}
