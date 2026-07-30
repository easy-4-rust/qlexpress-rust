//! Generate `impl QLExpressNativeType for T`.

use proc_macro2::TokenStream;
use quote::quote;

use crate::attrs::{ContainerAttrs, FieldSpec, ItemSpec};
/// 处理 generate 对应的领域职责。
/// 参数：`_ast`、`item`、`container`、`qlexpress_path`；返回：`TokenStream`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `generate`；Rust 侧按所有权与 `Result` 语义适配。
/// 对应 Java: 无（Rust 原生适配）。
pub fn generate(
    _ast: &syn::DeriveInput,
    item: &ItemSpec,
    container: &ContainerAttrs,
    qlexpress_path: &syn::Path,
) -> TokenStream {
    let type_name = container
        .name
        .clone()
        .unwrap_or_else(|| item.ident.to_string());
    let ident = &item.ident;

    let field_entries: Vec<TokenStream> = item
        .fields
        .iter()
        .filter(|f| !f.attrs.skip)
        .map(|f| gen_field_entry(f, qlexpress_path))
        .collect();

    let field_setter_entries: Vec<TokenStream> = item
        .fields
        .iter()
        .filter(|f| !f.attrs.skip && !f.attrs.readonly)
        .map(|f| gen_field_setter_entry(f, qlexpress_path))
        .collect();

    let field_alias_entries: Vec<TokenStream> = item
        .fields
        .iter()
        .filter(|f| !f.attrs.skip && !f.attrs.aliases.is_empty())
        .map(|f| {
            let name = f.ident.to_string();
            let aliases = &f.attrs.aliases;
            quote! {
                t.field_aliases.insert(
                    #name.to_string(),
                    vec![ #( #aliases.to_string() ),* ],
                );
            }
        })
        .collect();

    let mut combined = TokenStream::new();
    for entry in field_entries {
        combined.extend(entry);
    }
    for entry in field_setter_entries {
        combined.extend(entry);
    }
    for entry in field_alias_entries {
        combined.extend(entry);
    }

    // expose_fields 的 v1 行为由 `NativeObject::get_field` 默认实现已经覆盖:
    // 每个字段被 derive 后的 `impl NativeObject for T` 处理,
    // `obj.field` 走 `as_any().downcast_ref::<Self>()` 后访问字段。
    // 这里不再额外注册,只需保留属性机制供后续扩展。
    let expose_methods: Vec<TokenStream> = Vec::new();
    let mut expose_combined = TokenStream::new();
    for entry in expose_methods {
        expose_combined.extend(entry);
    }

    quote! {
        impl #qlexpress_path::runtime::member::QLExpressNativeType for #ident {
            const QL_TYPE_NAME: &'static str = #type_name;

            fn build_native_type()
                -> #qlexpress_path::runtime::member::NativeType
            {
                use ::std::collections::HashMap;
                use #qlexpress_path::runtime::member::{
                    NativeFieldGetter, NativeFieldSetter, NativeType,
                };

                let mut t = NativeType::named(#type_name);
                let fields: &mut HashMap<String, NativeFieldGetter> = &mut t.fields;
                #combined
                t
            }
        }
    }
}

fn gen_field_setter_entry(f: &FieldSpec, qlexpress_path: &syn::Path) -> TokenStream {
    let path = qlexpress_path;
    let name = f.ident.to_string();
    quote! {
        {
            let s: NativeFieldSetter = ::std::rc::Rc::new(
                move |
                    bean: & #path::runtime::value::DataValue,
                    value: & #path::runtime::value::DataValue,
                | -> bool {
                    let Some(cell) = bean.as_object_ref() else {
                        return false;
                    };
                    cell.borrow_mut().set_field(#name, value)
                }
            );
            t.field_setters.insert(#name.to_string(), s);
        }
    }
}

fn gen_field_entry(f: &FieldSpec, qlexpress_path: &syn::Path) -> TokenStream {
    let path = qlexpress_path;
    let name = f.ident.to_string();
    let field_ident = &f.ident;
    let value_expr = crate::convert::expr_to_data_value(&f.ty, quote!(me.#field_ident));
    quote! {
        {
            let g: NativeFieldGetter = ::std::rc::Rc::new(
                move |bean: & #path::runtime::value::DataValue|
                    -> Option< #path::runtime::value::DataValue>
                {
                    // Walk through the cell to get a `&Self`. The cell
                    // is `Rc<RefCell<dyn NativeObject>>` whose contents
                    // are a `Self`; borrow() returns a `Ref<dyn NativeObject>`
                    // which we downcast back to `&Self` via `as_any`.
                    let cell = bean.as_object_ref()?;
                    let native: &dyn #path::runtime::native_object::NativeObject =
                        &*cell.borrow();
                    let me: &Self = native.as_any().downcast_ref::<Self>()?;
                    Some({ #value_expr })
                }
            );
            fields.insert(#name.to_string(), g);
        }
    }
}
