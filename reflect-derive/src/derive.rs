use proc_macro::TokenStream;
use quote::ToTokens;
use reflect_schema::{Field, Schema, Type};
use syn::{parse_macro_input, spanned::Spanned};

use crate::{
    context::{Context, ReflectType},
    symbol::*,
};

pub(crate) fn derive_reflect(input: TokenStream, reflect_type: ReflectType) -> TokenStream {
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
            "failure to derive reflect::Input/reflect::Output while serde attributes compilation fails"
        );
    }

    let Some(serde_input) = serde_input else {
        // TODO when can this happen?
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflect::Input/reflect::Output without serde::Serialize and/or serde::Deserialize"
        );
    };

    let mut schema = Schema::new();
    let ctxt = Context::new(reflect_type);
    let input_type = visit_container(&ctxt, &serde_input, &mut schema);
    schema.types.push(input_type);
    if let Err(err) = ctxt.check() {
        proc_macro_error::abort!(err.span(), err.to_string());
    }

    // let ctxt = crate::context::Ctxt::new();
    // if ctxt.check().is_err() {
    //     proc_macro_error::abort!(
    //         input.ident,
    //         "failure to derive reflect::Input/reflect::Output while serde attributes compilation fails"
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

    if reflect_type == ReflectType::Input {
        TokenStream::from(quote::quote! {
            impl #name {
                fn reflect_input() -> reflect::Schema {
                    #tokenizable_schema
                }
            }
        })
    } else {
        TokenStream::from(quote::quote! {
            impl #name {
                fn reflect_output() -> reflect::Schema {
                    #tokenizable_schema
                }
            }
        })
    }
}

fn visit_container<'a>(
    cx: &Context,
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
//     cx: &crate::context::Ctxt,
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
    cx: &Context,
    field: &serde_derive_internals::ast::Field<'a>,
    schema: &mut Schema,
) -> reflect_schema::Field {
    let attrs = parse_field_attributes(cx, field.original);
    let field_type = match cx.reflect_type() {
        ReflectType::Input => attrs.input_type,
        ReflectType::Output => attrs.output_type,
    };

    let field_type = match field_type {
        Some(field_type) => field_type,
        None => visit_field_type(cx, &field.original.ty),
    };
    match field.member {
        syn::Member::Named(ref ident) => {
            // ident.type_id();
            // ident.
            // debug += &format!("field ident: {}\n", ident);
            Field::new(ident.to_string(), field_type)
        }
        syn::Member::Unnamed(ref index) => {
            // debug += &format!("field index: {}\n", index.index);
            Field::new(index.index.to_string(), field_type)
        }
    }
}

fn visit_field_type<'a>(
    cx: &Context,
    ty: &syn::Type,
    // schema: &mut Schema,
) -> reflect_schema::TypeRef {
    match ty {
        syn::Type::Path(path) => {
            // visit_type_path(cx, path, ty.span());
            if path.qself.is_some() {
                cx.error_spanned_by(
                    ty,
                    format_args!("reflect::Input/reflect::Output does not support qualified Self type reference"),
                );
            }
            path.path.segments.iter().for_each(|i| match i.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(_) => {
                    cx.error_spanned_by(
                        ty,
                        format_args!("reflect::Input/reflect::Output does not support generic field type"),
                    );
                }
                syn::PathArguments::Parenthesized(_) => {
                    cx.error_spanned_by(
                        ty,
                        format_args!(
                            "reflect::Input/reflect::Output does not support parenthesized field type path arguments"
                        ),
                    );
                }
            });
            return reflect_schema::TypeRef::new(path.to_token_stream().to_string());
        }
        syn::Type::Array(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support array field type"),
            );
            Default::default()
        }
        syn::Type::BareFn(path) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support bare function field type"
                ),
            );
            Default::default()
        }
        syn::Type::Group(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support group field type"),
            );
            Default::default()
        }
        syn::Type::ImplTrait(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support impl trait field type"
                ),
            );
            Default::default()
        }
        syn::Type::Infer(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support infer field type"),
            );
            Default::default()
        }
        syn::Type::Macro(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support macro field type"),
            );
            Default::default()
        }
        syn::Type::Never(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support never field type"),
            );
            Default::default()
        }
        syn::Type::Paren(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support paren field type"),
            );
            Default::default()
        }
        syn::Type::Ptr(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support pointer field type"),
            );
            Default::default()
        }
        syn::Type::Reference(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support reference field type"
                ),
            );
            Default::default()
        }
        syn::Type::Slice(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support slice field type"),
            );
            Default::default()
        }
        syn::Type::TraitObject(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support trait object field type"
                ),
            );
            Default::default()
        }
        syn::Type::Tuple(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support tuple field type"),
            );
            Default::default()
        }
        syn::Type::Verbatim(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support verbatim field type"),
            );
            Default::default()
        }
        _ => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support `{}` field type definition variant",
                    ty.to_token_stream().to_string()
                ),
            );
            Default::default()
        }
    }
}

struct ParsedFieldAttributes {
    pub input_type: Option<reflect_schema::TypeRef>,
    pub output_type: Option<reflect_schema::TypeRef>,
}

impl Default for ParsedFieldAttributes {
    fn default() -> Self {
        ParsedFieldAttributes {
            input_type: None,
            output_type: None,
        }
    }
}

// impl Field {
/// Extract out the `#[serde(...)]` attributes from a struct field.
fn parse_field_attributes(
    cx: &Context,
    // index: usize,
    field: &syn::Field,
    // attrs: Option<&Variant>,
    // container_default: &Default,
) -> ParsedFieldAttributes {
    let mut result = ParsedFieldAttributes::default();

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
            if meta.path == OUTPUT_TYPE {
                let supported_path = syn::Path::from(syn::Ident::new("u32", attr.span()));
                // #[reflect(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
                    if path.path != supported_path {
                        return Err(meta.error(format_args!(
                            "unknown reflect type reference path attribute"
                        )));
                    }
                    let referred_type = visit_field_type(
                        cx,
                        &syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }),
                    );
                    result.output_type = Some(referred_type);
                }
            } else if meta.path == INPUT_TYPE {
                // #[reflect(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    let referred_type = visit_field_type(
                        cx,
                        &syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }),
                    );
                    result.input_type = Some(referred_type);
                }
            } else if meta.path == TYPE {
                // #[reflect(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    let referred_type = visit_field_type(
                        cx,
                        &syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }),
                    );
                    result.output_type = Some(referred_type.clone());
                    result.input_type = Some(referred_type);
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

    result
}

fn parse_lit_into_expr_path(
    cx: &Context,
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
    cx: &Context,
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
