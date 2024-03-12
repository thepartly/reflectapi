use proc_macro::TokenStream;
use quote::ToTokens;
use reflect_schema::{Field, Struct, Type};

use crate::{
    context::{Context, ReflectType},
    symbol::*,
};

pub(crate) fn derive_reflect(input: TokenStream, reflect_type: ReflectType) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let type_ident = input.ident.clone();
    let type_generics = input.generics.clone();
    let type_generics_where = input.generics.where_clause.clone();

    let serde_context = serde_derive_internals::Ctxt::new();
    let serde_type_def = serde_derive_internals::ast::Container::from_ast(
        &serde_context,
        &input,
        serde_derive_internals::Derive::Deserialize,
    );
    if serde_context.check().is_err() {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflect::Input/reflect::Output while serde compilation fails"
        );
    }
    let Some(serde_input) = serde_type_def else {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflect::Input/reflect::Output while serde compilation fails"
        );
    };

    let reflected_context = Context::new(reflect_type);
    let reflected_type_def = visit_type(&reflected_context, &serde_input);
    let context_encounters = match reflected_context.check() {
        Err(err) => {
            proc_macro_error::abort!(err.span(), err.to_string());
        }
        Ok(unresolved_type_refs_to_syn_types) => unresolved_type_refs_to_syn_types,
    };

    let reflected_type_name = reflected_type_def.name();
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

    let reflected_type_generics = reflected_type_def
        .parameters()
        .map(|i| i.name().to_string())
        .collect::<Vec<_>>()
        .join(",");

    let mut fields_type_references_resolution_code = quote::quote! {};
    for (unresolved_field_type_ref, origin_field_syn_type) in
        context_encounters.fields_type_refs.into_iter()
    {
        // let unresolved_type_ref_generics = unresolved_type_ref.name.replace('>', "").split('<').skip(1).
        //     .parameters()
        //     .map(|i| i.name().to_string())
        //     .collect::<Vec<_>>()
        //     .join(",");
        // let unresolved_type_ref_generics = "ssss";
        let unresolved_field_type_ref =
            crate::tokenizable_schema::TokenizableTypeReference::new(&unresolved_field_type_ref);
        // let reflected_type_generics = reflected_type_def
        //     .parameters()
        //     .map(|i| i.name().to_string())
        //     .collect::<Vec<_>>()
        //     .join(",");
        fields_type_references_resolution_code.extend(quote::quote! {
            {
                let mut resolved_type_ref = <#origin_field_syn_type as #trait_ident>::#fn_reflect_type_ident(schema);
                // resolved_type_ref.name = format!("{}/{}/{}/{}/{}", resolved_type_ref.name, #unresolved_type_ref.name, stringify!(#type_generics), #reflected_type_generics, #unresolved_type_ref_generics);
                let unresolved_field_type_ref = #unresolved_field_type_ref;
                unresolved_field_type_ref.verify(&resolved_type_ref);
                println!("{}:  field resolved {:?} => {:?}", #reflected_type_name, unresolved_field_type_ref, resolved_type_ref);
                unresolved_to_resolved_fields_type_refs.insert(#unresolved_field_type_ref, resolved_type_ref);
            }
        });
    }

    let mut generics_type_references_resolution_code = quote::quote!();
    for p in reflected_type_def.parameters() {
        let p = syn::Ident::new(p.name(), proc_macro2::Span::call_site());
        generics_type_references_resolution_code.extend(quote::quote! {
            {
                let mut resolved_type_ref = <#p as #trait_ident>::#fn_reflect_type_ident(schema);
                // resolved_type_ref.name = format!("{}/{}/{}/{}/{}", resolved_type_ref.name, #unresolved_type_ref.name, stringify!(#type_generics), #reflected_type_generics, #unresolved_type_ref_generics);
                // let unresolved_field_type_ref = #unresolved_field_type_ref;
                // unresolved_field_type_ref.verify(&resolved_type_ref);
                println!("{}:  generic type resolved {:?}", #reflected_type_name, resolved_type_ref);
                parameters.push(resolved_type_ref);
            }
        });
    }

    let reflected_type_def = crate::tokenizable_schema::TokenizableType::new(&reflected_type_def);
    TokenStream::from(quote::quote! {
        impl #type_generics #trait_ident for #type_ident #type_generics #type_generics_where {
            fn #fn_reflect_type_ident(schema: &mut reflect::Schema) -> reflect::TypeReference {
                let resolved_type_name = format!("{}::{}", std::module_path!(), #reflected_type_name);
                let mut parameters = Vec::new();
                let mut reflected_type_def = #reflected_type_def;
                println!("{} resolving {}", resolved_type_name, reflected_type_def.name());
                // let reserve_name = format!("{}/{}", resolved_type_name, reflected_type_def.name());
                let reserve_name = format!("{}", resolved_type_name);
                if schema.reserve_type(reserve_name.as_ref()) {
                    reflected_type_def.rename(resolved_type_name.clone());

                    let mut unresolved_to_resolved_fields_type_refs = std::collections::HashMap::new();
                    #fields_type_references_resolution_code;
                    // reflected_type_def.set_description(format!("{:#?}", unresolved_to_resolved_fields_type_refs));
                    println!("{}: unresolved_to_resolved_fields_type_refs {:?}", resolved_type_name, unresolved_to_resolved_fields_type_refs);
                    let mut generic_to_specific_map = reflected_type_def.replace_type_references(&unresolved_to_resolved_fields_type_refs, schema);

                    println!("{}: generic_to_specific_map {:?}", resolved_type_name, generic_to_specific_map);
                    for p in reflected_type_def.parameters() {
                        parameters.push(
                            generic_to_specific_map
                                .remove(p.name())
                                .unwrap_or_else(|| p.name().into())
                        )
                    }

                    println!("finished resolving {:#?}", reflected_type_def);
                    schema.insert_type(reflected_type_def);
                } else {
                    println!("resolve conflict {}", resolved_type_name);
                    if let Some(reflected_type_def) = schema.get_type(resolved_type_name.as_ref()) {
                        println!("resolve conflict already defined {}", resolved_type_name);
                        // panic!("here 1");
                        for p in reflected_type_def.parameters() {
                            parameters.push(
                                reflect::TypeReference::from("TODO???")
                            )
                        }
                    } else {
                        println!("{}: resolve conflict already defined but not inserted, parameters: {:?}", resolved_type_name, reflected_type_def.parameters());
                        #generics_type_references_resolution_code
                        // // panic!("here 2 {:?}", reflected_type_def.parameters());
                        // // the case when the type is being built and there are circular references between types
                        // for p in reflected_type_def.parameters() {
                        //     parameters.push(
                        //         // reflect::TypeReference::new("reflect_demo::GenericStruct".into(), vec!["u8".into()])
                        //         // "u8".into()
                        //         p.name.clone().into()
                        //         // reflect::TypeReference::from("TODO 222???")
                        //     )
                        // }
                    }
                }
                let result = reflect::TypeReference::new(resolved_type_name.clone(), parameters);
                println!("{} returning resolved type refence {:?}", resolved_type_name, result);
                result
            }
        }

        impl #type_generics #type_ident #type_generics #type_generics_where {
            fn #fn_reflect_ident() -> reflect::Schema {
                let mut result = reflect::Schema::new();
                let resolved_type_ref = <Self as #trait_ident>::#fn_reflect_type_ident(&mut result);
                let resolved_type_def = result.get_type(resolved_type_ref.name.as_ref()).unwrap();
                if resolved_type_ref.parameters.len() != resolved_type_def.parameters().len() {
                    panic!("{} vs {} resolved_type_ref.parameters.len() != resolved_type_def.parameters().len()", resolved_type_ref.parameters.len(), resolved_type_def.parameters().len());
                }
                result.sort_types();
                result
            }
        }
    })
}

fn visit_type<'a>(cx: &Context, container: &serde_derive_internals::ast::Container<'a>) -> Type {
    let ident_name = container.ident.to_token_stream().to_string();

    // let mut result = String::new();
    let type_def = match &container.data {
        serde_derive_internals::ast::Data::Enum(_variants) => {
            // for variant in variants {
            //     result += &visit_variant(cx, variant, schema);
            // }
            unimplemented!("enum")
        }
        serde_derive_internals::ast::Data::Struct(_style, fields) => {
            let mut result = Struct::new(ident_name);
            for field in fields {
                result.fields.push(visit_field(cx, field));
            }
            container.generics.params.iter().for_each(|param| {
                if let syn::GenericParam::Type(type_param) = param {
                    // TODO here we discard a lot of info about generic type parameter
                    // should probably extend it in the future
                    result.parameters.push(type_param.ident.to_string().into());
                }
            });
            result.into()
        }
    };

    type_def
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
    let field_name = match field.member {
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref index) => index.index.to_string(),
    };
    let attrs = parse_field_attributes(cx, field.original);
    let (field_type, field_transform) = match cx.reflect_type() {
        ReflectType::Input => (attrs.input_type, attrs.input_transform),
        ReflectType::Output => (attrs.output_type, attrs.output_transform),
    };

    let field_type = match field_type {
        Some(field_type) => field_type,
        None => visit_field_type(cx, &field.original.ty),
    };

    let mut field_def = Field::new(field_name, field_type);
    field_def.transform_callback = field_transform;
    field_def
}

fn visit_field_type<'a>(cx: &Context, ty: &syn::Type) -> reflect_schema::TypeReference {
    let mut result: reflect_schema::TypeReference =
        ty.to_token_stream().to_string().replace(' ', "").into();
    match ty {
        syn::Type::Path(path) => {
            if path.qself.is_some() {
                cx.impl_error(
                    ty,
                    format_args!("reflect::Input/reflect::Output does not support qualified Self type reference"),
                );
            }
            path.path.segments.iter().for_each(|i| match &i.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(args) => {
                    for i in args.args.iter() {
                        match i {
                            syn::GenericArgument::Type(ty) => {
                                result.parameters.push(ty.to_token_stream().to_string().into());
                                // TODO what to do here?
                                // visit_field_type(cx, ty);
                            }
                            // syn::GenericArgument::Binding(binding) => {
                            //     cx.impl_error(
                            //         ty,
                            //         format_args!(
                            //             "reflect::Input/reflect::Output does not support generic field type"
                            //         ),
                            //     );
                            // }
                            // syn::GenericArgument::Constraint(constraint) => {
                            //     cx.impl_error(
                            //         ty,
                            //         format_args!(
                            //             "reflect::Input/reflect::Output does not support generic field type"
                            //         ),
                            //     );
                            // }
                            // syn::GenericArgument::Const(constant) => {
                            //     cx.impl_error(
                            //         ty,
                            //         format_args!(
                            //             "reflect::Input/reflect::Output does not support generic field type"
                            //         ),
                            //     );
                            // }
                            // syn::GenericArgument::Lifetime(lifetime) => {
                            //     cx.impl_error(
                            //         ty,
                            //         format_args!(
                            //             "reflect::Input/reflect::Output does not support generic field type"
                            //         ),
                            //     );
                            // }
                            // syn::GenericArgument::Verbatim(_) => {
                            //     cx.impl_error(
                            //         ty,
                            //         format_args!(
                            //             "reflect::Input/reflect::Output does not support generic field type"
                            //         ),
                            //     );
                            // }
                            _ => {}
                        }
                    }
                    // cx.impl_error(
                    //     ty,
                    //     format_args!("reflect::Input/reflect::Output does not support generic field type"),
                    // );
                }
                syn::PathArguments::Parenthesized(_) => {
                    cx.impl_error(
                        ty,
                        format_args!(
                            "reflect::Input/reflect::Output does not support parenthesized field type path arguments"
                        ),
                    );
                }
            });
            // let tr = path
            //     .path
            //     .segments
            //     .iter()
            //     .map(|i| i.ident.to_string())
            //     .collect::<Vec<_>>()
            //     .join("::");
            // let mut r = reflect_schema::TypeReference::new(tr);
            // r._debug(Some(format!("{}/{}", result.name, r.name.clone())));
            // return r;
            // result._debug(Some(tr));
        }
        syn::Type::Array(_) => {
            // cx.impl_error(
            //     ty,
            //     format_args!("reflect::Input/reflect::Output does not support array field type"),
            // );
        }
        syn::Type::BareFn(_path) => {
            cx.impl_error(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support bare function field type"
                ),
            );
        }
        syn::Type::Group(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support group field type"),
            );
        }
        syn::Type::ImplTrait(_) => {
            cx.impl_error(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support impl trait field type"
                ),
            );
        }
        syn::Type::Infer(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support infer field type"),
            );
        }
        syn::Type::Macro(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support macro field type"),
            );
        }
        syn::Type::Never(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support never field type"),
            );
        }
        syn::Type::Paren(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support paren field type"),
            );
        }
        syn::Type::Ptr(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support pointer field type"),
            );
        }
        syn::Type::Reference(_) => {
            cx.impl_error(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support reference field type"
                ),
            );
        }
        syn::Type::Slice(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support slice field type"),
            );
        }
        syn::Type::TraitObject(_) => {
            cx.impl_error(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support trait object field type"
                ),
            );
        }
        syn::Type::Tuple(_) => {
            // cx.impl_error(
            //     ty,
            //     format_args!("reflect::Input/reflect::Output does not support tuple field type"),
            // );
        }
        syn::Type::Verbatim(_) => {
            cx.impl_error(
                ty,
                format_args!("reflect::Input/reflect::Output does not support verbatim field type"),
            );
        }
        _ => {
            cx.impl_error(
                ty,
                format_args!(
                    "reflect::Input/reflect::Output does not support `{}` field type definition variant",
                    ty.to_token_stream().to_string()
                ),
            );
        }
    }
    cx.encountered_type_ref(result.clone(), ty.clone());
    result
}

struct ParsedFieldAttributes {
    pub input_type: Option<reflect_schema::TypeReference>,
    pub output_type: Option<reflect_schema::TypeReference>,
    pub input_transform: String,
    pub output_transform: String,
}

impl Default for ParsedFieldAttributes {
    fn default() -> Self {
        ParsedFieldAttributes {
            input_type: None,
            output_type: None,
            input_transform: String::new(),
            output_transform: String::new(),
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
                    if cx.reflect_type() == ReflectType::Output {
                        let referred_type = visit_field_type(
                            cx,
                            &syn::Type::Path(syn::TypePath {
                                qself: path.qself,
                                path: path.path,
                            }),
                        );
                        result.output_type = Some(referred_type);
                    }
                }
            } else if meta.path == INPUT_TYPE {
                // #[reflect(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflect_type() == ReflectType::Input {
                        let referred_type = visit_field_type(
                            cx,
                            &syn::Type::Path(syn::TypePath {
                                qself: path.qself,
                                path: path.path,
                            }),
                        );
                        result.input_type = Some(referred_type);
                    }
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
            } else if meta.path == OUTPUT_TRANSFORM {
                // #[reflect(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
                    if cx.reflect_type() == ReflectType::Output {
                        result.output_transform = path.to_token_stream().to_string();
                    }
                }
            } else if meta.path == INPUT_TRANSFORM {
                // #[reflect(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflect_type() == ReflectType::Input {
                        result.input_transform = path.to_token_stream().to_string();
                    }
                }
            } else if meta.path == TRANSFORM {
                // #[reflect(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    result.output_transform = path.to_token_stream().to_string();
                    result.input_transform = path.to_token_stream().to_string();
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
            cx.impl_error(
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
            cx.impl_error(
                lit,
                format!("unexpected suffix `{}` on string literal", suffix),
            );
        }
        Ok(Some(lit.clone()))
    } else {
        cx.impl_error(
            expr,
            format!(
                "expected serde {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Ok(None)
    }
}

// fn type_ref_to_syn_path(str: &str) -> syn::TypePath {
//     let (leading_colon, path_parts) = if str.starts_with("::") {
//         (
//             Some(syn::Token![::](proc_macro2::Span::call_site())),
//             str.split("::").skip(1).collect::<Vec<_>>(),
//         )
//     } else {
//         (None, str.split("::").collect::<Vec<_>>())
//     };
//     let segments = path_parts
//         .iter()
//         .map(|s| syn::PathSegment {
//             ident: syn::Ident::new(s, proc_macro2::Span::call_site()),
//             arguments: syn::PathArguments::None,
//         })
//         .collect();
//     let path = syn::Path {
//         leading_colon,
//         segments,
//     };
//     syn::TypePath { qself: None, path }
// }
