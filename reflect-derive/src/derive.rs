use proc_macro::TokenStream;
use quote::ToTokens;
use reflect_schema::{Field, Schema, Type};
use syn::{parse_macro_input, spanned::Spanned, token};

use crate::symbol::*;

pub(crate) fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let input_dump = format!("{:#?}", input);
    let name = input.ident.clone();

    let ctxt = serde_derive_internals::Ctxt::new();
    let serde_input = serde_derive_internals::ast::Container::from_ast(
        &ctxt,
        &input,
        serde_derive_internals::Derive::Deserialize,
    );
    if ctxt.check().is_err() {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive Reflect while serde attributes compilation fails"
        );
    }

    let Some(serde_input) = serde_input else {
        // TODO when can this happen?
        proc_macro_error::abort!(
            input.ident,
            "failure to derive Reflect without serde::Serialize and/or serde::Deserialize"
        );
    };

    let mut schema = Schema::new();
    let ctxt = serde_derive_internals::Ctxt::new();
    let input_type = visit_container(&ctxt, &serde_input, &mut schema);
    schema.types.push(input_type);
    if let Err(err) = ctxt.check() {
        proc_macro_error::abort!(err.span(), err.to_string());
    }

    // let ctxt = serde_derive_internals::Ctxt::new();
    // if ctxt.check().is_err() {
    //     proc_macro_error::abort!(
    //         input.ident,
    //         "failure to derive Reflect while serde attributes compilation fails"
    //     );
    // }
    // visit_container(&ctxt, serde_input);

    // println!("{:#?}", input_dump);

    // proc_macro_error::abort!(input.ident, input_dump);

    // let serde_dump = format!("{:#?}", result);

    // expand::my_derive(input)
    //     .unwrap_or_else(|err| err.to_compile_error())
    //     .into()

    // .ok_or(())
    // .and_then(|serde| Self::from_serde(&ctxt, serde));

    // ctxt.check()
    //     .map(|_| result.expect("from_ast set no errors on Ctxt, so should have returned Ok"))

    // let schema_json = serde_json::to_string_pretty(&schema).unwrap();

    let tokenizable_schema = crate::tokenizable_schema::TokenizableSchema::new(schema);

    TokenStream::from(quote::quote! {
        impl #name {
            // fn reflect() -> String {
            //     String::from(#input_dump)
            // }
            // fn reflect_debug(&self) -> String {
            //     String::from(#reflect_input)
            // }
            fn reflect() -> reflect::Schema {
                #tokenizable_schema
            }
        }
    })
}

fn visit_container<'a>(
    cx: &serde_derive_internals::Ctxt,
    container: &serde_derive_internals::ast::Container<'a>,
    schema: &mut Schema,
) -> Type {
    let mut result = Type::new(container.ident.to_string());

    // let mut result = String::new();
    match &container.data {
        serde_derive_internals::ast::Data::Enum(variants) => {
            // for variant in variants {
            //     result += &visit_variant(cx, variant, schema);
            // }
        }
        serde_derive_internals::ast::Data::Struct(style, fields) => {
            for field in fields {
                result.fields.push(visit_field(cx, field, schema));
            }
        }
    }
    result
}

// fn visit_variant<'a>(
//     cx: &serde_derive_internals::Ctxt,
//     variant: &serde_derive_internals::ast::Variant<'a>,
//     schema: &mut Schema,
// ) -> String {
//     let mut result = String::new();
//     for field in &variant.fields {
//         result += &visit_field(cx, field, schema);
//     }
//     result
// }

fn visit_field<'a>(
    cx: &serde_derive_internals::Ctxt,
    field: &serde_derive_internals::ast::Field<'a>,
    schema: &mut Schema,
) -> reflect_schema::Field {
    let attrs = &field.original.attrs;

    // field.original.attrs
    // let mut debug = String::new();
    // let field_dump = format!("{:#?}", field.member);
    // debug += &format!("field member: {}\n", field_dump);
    // debug += &format!("field attrs: {:#?}\n", attrs);

    field_from_ast(cx, field.original);

    match field.member {
        syn::Member::Named(ref ident) => {
            // ident.type_id();
            // ident.
            // debug += &format!("field ident: {}\n", ident);
            Field::new(ident.to_string(), format!("{:?}", field.original.ty))
        }
        syn::Member::Unnamed(ref index) => {
            // debug += &format!("field index: {}\n", index.index);
            Field::new(index.index.to_string(), format!("{:?}", field.original.ty))
        }
    }
}

// impl Field {
/// Extract out the `#[serde(...)]` attributes from a struct field.
fn field_from_ast(
    cx: &serde_derive_internals::Ctxt,
    // index: usize,
    field: &syn::Field,
    // attrs: Option<&Variant>,
    // container_default: &Default,
) -> () {
    // let mut serialize_type = "serialize_type";
    // let mut deserialize_type = "deserialize_type";

    // let ident = match &field.ident {
    //     Some(ident) => unraw(ident),
    //     None => index.to_string(),
    // };

    // if let Some(borrow_attribute) = attrs.and_then(|variant| variant.borrow.as_ref()) {
    //     if let Ok(borrowable) = borrowable_lifetimes(cx, &ident, field) {
    //         if let Some(lifetimes) = &borrow_attribute.lifetimes {
    //             for lifetime in lifetimes {
    //                 if !borrowable.contains(lifetime) {
    //                     let msg =
    //                         format!("field `{}` does not have lifetime {}", ident, lifetime);
    //                     cx.error_spanned_by(field, msg);
    //                 }
    //             }
    //             borrowed_lifetimes.set(&borrow_attribute.path, lifetimes.clone());
    //         } else {
    //             borrowed_lifetimes.set(&borrow_attribute.path, borrowable);
    //         }
    //     }
    // }

    for attr in &field.attrs {
        if attr.path() != REFLECT {
            continue;
        }

        if let syn::Meta::List(meta) = &attr.meta {
            if meta.tokens.is_empty() {
                continue;
            }
        }

        if let Err(err) = attr.parse_nested_meta(|meta| {
            if meta.path == SERIALIZE_TYPE {
                let supported_path = syn::Path::from(syn::Ident::new("u32", attr.span()));
                // #[reflect(serialize_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, SERIALIZE_TYPE, &meta)? {
                    if path.path != supported_path {
                        return Err(meta.error(format_args!(
                            "unknown reflect type reference path attribute"
                        )));
                    }
                    // serialize_with.set(&meta.path, path);
                }
            } else if meta.path == DESERIALIZE_TYPE {
                // #[reflect(deserialize_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, DESERIALIZE_TYPE, &meta)? {
                    // deserialize_with.set(&meta.path, path);
                }
            } else if meta.path == TYPE {
                // #[reflect(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    // let mut ser_path = path.clone();
                    // ser_path
                    //     .path
                    //     .segments
                    //     .push(Ident::new("serialize", Span::call_site()).into());
                    // serialize_with.set(&meta.path, ser_path);
                    // let mut de_path = path;
                    // de_path
                    //     .path
                    //     .segments
                    //     .push(Ident::new("deserialize", Span::call_site()).into());
                    // deserialize_with.set(&meta.path, de_path);
                }
            } else {
                let path = meta.path.to_token_stream().to_string().replace(' ', "");
                return Err(meta.error(format_args!("unknown reflect field attribute `{}`", path)));
            }
            Ok(())
        }) {
            cx.syn_error(err);
        }
    }

    // // Is skip_deserializing, initialize the field to Default::default() unless a
    // // different default is specified by `#[serde(default = "...")]` on
    // // ourselves or our container (e.g. the struct we are in).
    // if let Default::None = *container_default {
    //     if skip_deserializing.0.value.is_some() {
    //         default.set_if_none(Default::Default);
    //     }
    // }

    // let mut borrowed_lifetimes = borrowed_lifetimes.get().unwrap_or_default();
    // if !borrowed_lifetimes.is_empty() {
    //     // Cow<str> and Cow<[u8]> never borrow by default:
    //     //
    //     //     impl<'de, 'a, T: ?Sized> Deserialize<'de> for Cow<'a, T>
    //     //
    //     // A #[serde(borrow)] attribute enables borrowing that corresponds
    //     // roughly to these impls:
    //     //
    //     //     impl<'de: 'a, 'a> Deserialize<'de> for Cow<'a, str>
    //     //     impl<'de: 'a, 'a> Deserialize<'de> for Cow<'a, [u8]>
    //     if is_cow(&field.ty, is_str) {
    //         let mut path = syn::Path {
    //             leading_colon: None,
    //             segments: Punctuated::new(),
    //         };
    //         let span = Span::call_site();
    //         path.segments.push(Ident::new("_serde", span).into());
    //         path.segments.push(Ident::new("__private", span).into());
    //         path.segments.push(Ident::new("de", span).into());
    //         path.segments
    //             .push(Ident::new("borrow_cow_str", span).into());
    //         let expr = syn::ExprPath {
    //             attrs: Vec::new(),
    //             qself: None,
    //             path,
    //         };
    //         deserialize_with.set_if_none(expr);
    //     } else if is_cow(&field.ty, is_slice_u8) {
    //         let mut path = syn::Path {
    //             leading_colon: None,
    //             segments: Punctuated::new(),
    //         };
    //         let span = Span::call_site();
    //         path.segments.push(Ident::new("_serde", span).into());
    //         path.segments.push(Ident::new("__private", span).into());
    //         path.segments.push(Ident::new("de", span).into());
    //         path.segments
    //             .push(Ident::new("borrow_cow_bytes", span).into());
    //         let expr = syn::ExprPath {
    //             attrs: Vec::new(),
    //             qself: None,
    //             path,
    //         };
    //         deserialize_with.set_if_none(expr);
    //     }
    // } else if is_implicitly_borrowed(&field.ty) {
    //     // Types &str and &[u8] are always implicitly borrowed. No need for
    //     // a #[serde(borrow)].
    //     collect_lifetimes(&field.ty, &mut borrowed_lifetimes);
    // }

    // Field {
    //     name: Name::from_attrs(ident, ser_name, de_name, Some(de_aliases)),
    //     skip_serializing: skip_serializing.get(),
    //     skip_deserializing: skip_deserializing.get(),
    //     skip_serializing_if: skip_serializing_if.get(),
    //     default: default.get().unwrap_or(Default::None),
    //     serialize_with: serialize_with.get(),
    //     deserialize_with: deserialize_with.get(),
    //     ser_bound: ser_bound.get(),
    //     de_bound: de_bound.get(),
    //     borrowed_lifetimes,
    //     getter: getter.get(),
    //     flatten: flatten.get(),
    //     transparent: false,
    // }
}

fn parse_lit_into_expr_path(
    cx: &serde_derive_internals::Ctxt,
    attr_name: Symbol,
    meta: &syn::meta::ParseNestedMeta,
) -> syn::Result<Option<syn::ExprPath>> {
    let string = match get_lit_str(cx, attr_name, attr_name, meta)? {
        Some(string) => string,
        None => return Ok(None),
    };

    Ok(match string.parse() {
        Ok(expr) => Some(expr),
        Err(_) => {
            cx.error_spanned_by(
                &string,
                format!("failed to parse type reference path: {:?}", string.value()),
            );
            None
        }
    })
}

fn get_lit_str(
    cx: &serde_derive_internals::Ctxt,
    attr_name: Symbol,
    meta_item_name: Symbol,
    meta: &syn::meta::ParseNestedMeta,
) -> syn::Result<Option<syn::LitStr>> {
    let expr: syn::Expr = meta.value()?.parse()?;
    let mut value = &expr;
    while let syn::Expr::Group(e) = value {
        value = &e.expr;
    }
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = value
    {
        let suffix = lit.suffix();
        if !suffix.is_empty() {
            cx.error_spanned_by(
                lit,
                format!("unexpected suffix `{}` on string literal", suffix),
            );
        }
        Ok(Some(lit.clone()))
    } else {
        cx.error_spanned_by(
            expr,
            format!(
                "expected serde {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Ok(None)
    }
}
