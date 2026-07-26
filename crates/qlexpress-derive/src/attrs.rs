//! Parsing of `#[qlexpress(...)]` helper-attribute lists attached to
//! container, fields and methods.
//!
//! The grammar is intentionally tiny — see `lib.rs` for the full doc.

use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Meta, Result, Token};

/// Attributes applied to the type as a whole.
#[derive(Debug, Default)]
pub struct ContainerAttrs {
    pub name: Option<String>,
    pub no_native_object: bool,
    /// v1 新增:把每个 pub 字段也注册为同名方法,让 `obj.field` 既走
    /// `NativeType.fields` 也走 `methods`。对齐 Java 直接 public field 访问
    /// (`Issue318Test`)。
    pub expose_fields: bool,
}

impl ContainerAttrs {
    pub fn from_ast(ast: &syn::DeriveInput) -> Result<Self> {
        let mut out = ContainerAttrs::default();
        for attr in &ast.attrs {
            for arg in parse_qlexpress_args(attr)? {
                apply_container_arg(&mut out, arg)?;
            }
        }
        Ok(out)
    }
}

fn apply_container_arg(out: &mut ContainerAttrs, arg: QlexAttr) -> Result<()> {
    match arg {
        QlexAttr::Flag(name) if name == "no_native_object" => {
            out.no_native_object = true;
            Ok(())
        }
        QlexAttr::Flag(name) if name == "expose_fields" => {
            out.expose_fields = true;
            Ok(())
        }
        QlexAttr::KeyValue { key, value } if key == "name" => {
            out.name = Some(value);
            Ok(())
        }
        other => Err(syn::Error::new_spanned(
            other,
            "unsupported #[qlexpress(...)] attribute on type",
        )),
    }
}

/// Per-field attributes.
#[derive(Debug, Default)]
pub struct FieldAttrs {
    pub skip: bool,
    pub readonly: bool,
    pub aliases: Vec<String>,
}

impl FieldAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut out = FieldAttrs::default();
        for attr in attrs {
            for arg in parse_qlexpress_args(attr)? {
                apply_field_arg(&mut out, arg)?;
            }
        }
        Ok(out)
    }
}

fn apply_field_arg(out: &mut FieldAttrs, arg: QlexAttr) -> Result<()> {
    match arg {
        QlexAttr::Flag(name) if name == "skip" => out.skip = true,
        QlexAttr::Flag(name) if name == "readonly" => out.readonly = true,
        QlexAttr::List { key, values } if key == "alias" => {
            out.aliases.extend(values);
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "unsupported #[qlexpress(...)] attribute on field",
            ));
        }
    }
    Ok(())
}

/// A single argument inside one `#[qlexpress(...)]` attribute.
#[derive(Clone)]
pub enum QlexAttr {
    Flag(String),
    KeyValue { key: String, value: String },
    List { key: String, values: Vec<String> },
}

impl Parse for QlexAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: syn::Ident = input.parse()?;
        let key_str = key.to_string();
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let lit: syn::LitStr = input.parse()?;
            Ok(QlexAttr::KeyValue {
                key: key_str,
                value: lit.value(),
            })
        } else if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let mut values = Vec::new();
            while !content.is_empty() {
                let lit: syn::LitStr = content.parse()?;
                values.push(lit.value());
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
            Ok(QlexAttr::List {
                key: key_str,
                values,
            })
        } else {
            Ok(QlexAttr::Flag(key_str))
        }
    }
}

impl std::fmt::Display for QlexAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QlexAttr::Flag(s) => write!(f, "{s}"),
            QlexAttr::KeyValue { key, value } => write!(f, "{key} = {value}"),
            QlexAttr::List { key, values } => write!(f, "{key}({})", values.join(", ")),
        }
    }
}

impl quote::ToTokens for QlexAttr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = self.to_string();
        tokens.extend(quote::quote!(#s));
    }
}

fn parse_qlexpress_args(attr: &Attribute) -> Result<Vec<QlexAttr>> {
    if !attr.path().is_ident("qlexpress") {
        return Ok(Vec::new());
    }
    match &attr.meta {
        Meta::List(list) => {
            let parser = syn::punctuated::Punctuated::<QlexAttr, Token![,]>::parse_terminated;
            let punct = list.parse_args_with(parser)?;
            Ok(punct.into_iter().collect())
        }
        Meta::Path(_) => Err(syn::Error::new_spanned(
            attr,
            "#[qlexpress] requires arguments, e.g. #[qlexpress(name = \"foo\")]",
        )),
        Meta::NameValue(_) => Err(syn::Error::new_spanned(
            attr,
            "#[qlexpress(...)] must be a list, not key=value directly",
        )),
    }
}

/// Aggregated information we need to generate code for one type.
pub struct ItemSpec {
    pub ident: syn::Ident,
    pub fields: Vec<FieldSpec>,
}

pub struct FieldSpec {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub attrs: FieldAttrs,
}

impl ItemSpec {
    pub fn from_ast(ast: &syn::DeriveInput) -> Result<Self> {
        let syn::Data::Struct(data) = &ast.data else {
            return Err(syn::Error::new_spanned(
                ast,
                "QLExpressType derive only supports structs in v1",
            ));
        };
        let syn::Fields::Named(named) = &data.fields else {
            return Err(syn::Error::new_spanned(
                ast,
                "QLExpressType derive requires a struct with named fields",
            ));
        };
        if !ast.generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                &ast.generics,
                "QLExpressType derive does not support generic types in v1",
            ));
        }

        let mut fields = Vec::new();
        for f in &named.named {
            let ident = f
                .ident
                .clone()
                .ok_or_else(|| syn::Error::new_spanned(f, "field must be named"))?;
            let attrs = FieldAttrs::from_attrs(&f.attrs)?;
            fields.push(FieldSpec {
                ident,
                ty: f.ty.clone(),
                attrs,
            });
        }

        Ok(ItemSpec {
            ident: ast.ident.clone(),
            fields,
        })
    }
}
