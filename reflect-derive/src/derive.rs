use proc_macro::TokenStream;
use quote::ToTokens;
use reflect_schema::{Field, Type};

use crate::{
    context::{Context, ReflectType},
    symbol::*,
};

pub(crate) fn derive_reflect(input: TokenStream, reflect_type: ReflectType) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
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
            "failure to derive reflect::Input/reflect::Output while serde compilation fails"
        );
    }
    let Some(serde_input) = serde_input else {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflect::Input/reflect::Output while serde compilation fails"
        );
    };

    let ctxt = Context::new(reflect_type);
    let type_schema = visit_type(&ctxt, &serde_input);
    if let Err(err) = ctxt.check() {
        proc_macro_error::abort!(err.span(), err.to_string());
    }

    let reflect_type_name = type_schema.name.clone();
    let reflect_type_refs = type_schema.fields();
    let (fn_reflect_ident, fn_reflect_type_ident, trait_ident) = match reflect_type {
        ReflectType::Input => (
            quote::quote!(reflect_input),
            quote::quote!(reflect_input_type),
            quote::quote!(reflect::Input),
        ),
        ReflectType::Output => (
            quote::quote!(reflect_output),
            quote::quote!(reflect_output_type),
            quote::quote!(reflect::Output),
        ),
    };

    let mut reflect_type_refs_processing = quote::quote! {};
    for type_ref in reflect_type_refs.into_iter() {
        let type_ref_name = type_ref.type_ref.name.as_str();
        let type_ref_ident = type_ref_to_syn_path(type_ref_name);
        reflect_type_refs_processing.extend(quote::quote! {
            {
                let reflectable_type_name = <#type_ref_ident as #trait_ident>::#fn_reflect_type_ident(schema);
                type_refs_map.insert(String::from(#type_ref_name), reflectable_type_name);
            }
        });
    }
    let tokenizable_type_schema = crate::tokenizable_schema::TokenizableType::new(type_schema);
    TokenStream::from(quote::quote! {
        impl #trait_ident for #name {
            fn #fn_reflect_type_ident(schema: &mut reflect::Schema) -> String {
                let full_type_name = format!("{}::{}", std::module_path!(), #reflect_type_name);
                if schema.has_type(&full_type_name) {
                    return full_type_name;
                }
                let mut result = #tokenizable_type_schema;
                result.name = full_type_name.clone();
                schema.reserve_type(result.name.clone());
                let mut type_refs_map = std::collections::HashMap::new();
                #reflect_type_refs_processing;
                result.remap_type_refs(&type_refs_map);
                schema.insert_type(result);
                full_type_name
            }
        }

        impl #name {
            fn #fn_reflect_ident() -> reflect::Schema {
                let mut result = reflect::Schema::new();
                <Self as #trait_ident>::#fn_reflect_type_ident(&mut result);
                result.sort_types();
                result
            }
        }
    })
}

fn visit_type<'a>(cx: &Context, container: &serde_derive_internals::ast::Container<'a>) -> Type {
    let ident_name = container.ident.to_token_stream().to_string();
    let mut result = Type::new(ident_name);

    // let mut result = String::new();
    match &container.data {
        serde_derive_internals::ast::Data::Enum(_variants) => {
            // for variant in variants {
            //     result += &visit_variant(cx, variant, schema);
            // }
        }
        serde_derive_internals::ast::Data::Struct(_style, fields) => {
            for field in fields {
                result.fields.push(visit_field(cx, field));
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
        syn::Member::Named(ref ident) => Field::new(ident.to_string(), field_type),
        syn::Member::Unnamed(ref index) => Field::new(index.index.to_string(), field_type),
    }
}

fn visit_field_type<'a>(cx: &Context, ty: &syn::Type) -> reflect_schema::TypeRef {
    match ty {
        syn::Type::Path(path) => {
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
            let tr = path
                .path
                .segments
                .iter()
                .map(|i| i.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            return reflect_schema::TypeRef::new(tr);
        }
        syn::Type::Array(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support array field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::BareFn(_path) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support bare function field type"
                ),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Group(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support group field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::ImplTrait(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support impl trait field type"
                ),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Infer(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support infer field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Macro(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support macro field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Never(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support never field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Paren(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support paren field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Ptr(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support pointer field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Reference(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support reference field type"
                ),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Slice(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support slice field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::TraitObject(_) => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support trait object field type"
                ),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Tuple(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support tuple field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        syn::Type::Verbatim(_) => {
            cx.error_spanned_by(
                ty,
                format_args!("reflect::Input/reflect::Output does not support verbatim field type"),
            );
            reflect_schema::TypeRef::invalid()
        }
        _ => {
            cx.error_spanned_by(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support `{}` field type definition variant",
                    ty.to_token_stream().to_string()
                ),
            );
            reflect_schema::TypeRef::invalid()
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

/// Extract out the `#[reflect(...)]` attributes from a struct field.
fn parse_field_attributes(cx: &Context, field: &syn::Field) -> ParsedFieldAttributes {
    let mut result = ParsedFieldAttributes::default();

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
                // #[reflect(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
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
    result
}

fn parse_lit_into_expr_path(
    cx: &Context,
    attr_name: Symbol,
    meta: &syn::meta::ParseNestedMeta,
) -> syn::Result<Option<syn::ExprPath>> {
    let string = match parse_lit_str(cx, attr_name, attr_name, meta)? {
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

fn parse_lit_str(
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

fn type_ref_to_syn_path(str: &str) -> syn::Path {
    let (leading_colon, path_parts) = if str.starts_with("::") {
        (
            Some(syn::Token![::](proc_macro2::Span::call_site())),
            str.split("::").skip(1).collect::<Vec<_>>(),
        )
    } else {
        (None, str.split("::").collect::<Vec<_>>())
    };
    let segments = path_parts
        .iter()
        .map(|s| syn::PathSegment {
            ident: syn::Ident::new(s, proc_macro2::Span::call_site()),
            arguments: syn::PathArguments::None,
        })
        .collect();
    syn::Path {
        leading_colon,
        segments,
    }
}
