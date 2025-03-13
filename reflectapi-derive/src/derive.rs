use proc_macro::TokenStream;
use quote::ToTokens;
use reflectapi_schema::{Enum, Field, Fields, Struct, Type, TypeParameter, Variant};
use serde_derive_internals::ast;
use syn::parse_quote;

use crate::{
    context::{Context, ReflectType},
    parser::{
        naive_parse_as_type_reference, parse_doc_attributes, parse_field_attributes,
        parse_type_attributes, parse_variant_attributes,
    },
};

pub(crate) fn derive_reflect(input: TokenStream, reflectapi_type: ReflectType) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let type_ident = input.ident.clone();
    let mut type_generics = input.generics.clone();

    for param in type_generics.type_params_mut() {
        // `impl` generics are not allowed to have defaults.
        param.default = None;

        // Bound each generic by `reflectapi::Input` or `reflectapi::Output`.
        param.bounds.push(match reflectapi_type {
            ReflectType::Input => parse_quote!(reflectapi::Input),
            ReflectType::Output => parse_quote!(reflectapi::Output),
        });
    }
    let type_generics_where = &input.generics.where_clause;

    let serde_context = serde_derive_internals::Ctxt::new();
    let serde_type_def = ast::Container::from_ast(
        &serde_context,
        &input,
        serde_derive_internals::Derive::Deserialize,
    );
    if serde_context.check().is_err() {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflectapi::Input/reflectapi::Output while serde compilation fails"
        );
    }
    let Some(serde_input) = serde_type_def else {
        proc_macro_error::abort!(
            input.ident,
            "failure to derive reflectapi::Input/reflectapi::Output while serde compilation fails"
        );
    };

    let reflected_context = Context::new(reflectapi_type);
    let reflected_type_def = visit_type(&reflected_context, &serde_input);
    let context_encounters = match reflected_context.check() {
        Err(err) => {
            proc_macro_error::abort!(err.span(), err.to_string());
        }
        Ok(unresolved_type_refs_to_syn_types) => unresolved_type_refs_to_syn_types,
    };

    let reflected_type_name = reflected_type_def.name();
    let (fn_reflectapi_ident, fn_reflectapi_type_ident, trait_ident) = match reflectapi_type {
        ReflectType::Input => (
            quote::quote!(reflectapi_input),
            quote::quote!(reflectapi_input_type),
            quote::quote!(reflectapi::Input),
        ),
        ReflectType::Output => (
            quote::quote!(reflectapi_output),
            quote::quote!(reflectapi_output_type),
            quote::quote!(reflectapi::Output),
        ),
    };

    let type_generics_idents = context_encounters
        .generics
        .iter()
        .map(|(_, ident)| ident)
        .collect::<Vec<_>>();
    let type_generics_idents_code = if type_generics_idents.is_empty() {
        quote::quote!()
    } else {
        quote::quote! {
            <#(#type_generics_idents),*>
        }
    };

    let mut fields_type_references_resolution_code = quote::quote! {};
    for (unresolved_field_type_ref, origin_field_syn_type) in context_encounters.fields.into_iter()
    {
        let unresolved_field_type_ref =
            crate::tokenizable_schema::TokenizableTypeReference::new(&unresolved_field_type_ref);
        fields_type_references_resolution_code.extend(quote::quote! {
            {
                let mut resolved_type_ref = <#origin_field_syn_type as #trait_ident>::#fn_reflectapi_type_ident(schema);
                let unresolved_field_type_ref = #unresolved_field_type_ref;
                unresolved_to_resolved_fields_type_refs.insert(#unresolved_field_type_ref, resolved_type_ref);
            }
        });
    }

    let mut generics_type_references_resolution_code = quote::quote!();
    for (_, origin_type_param) in context_encounters.generics.into_iter() {
        generics_type_references_resolution_code.extend(quote::quote! {
            {
                arguments.push(<#origin_type_param as #trait_ident>::#fn_reflectapi_type_ident(schema));
            }
        });
    }

    let reflected_type_def = crate::tokenizable_schema::TokenizableType::new(&reflected_type_def);
    TokenStream::from(quote::quote! {
        #[allow(unused_doc_comments)]
        impl #type_generics #trait_ident for #type_ident #type_generics_idents_code #type_generics_where {
            fn #fn_reflectapi_type_ident(schema: &mut reflectapi::Typespace) -> reflectapi::TypeReference {
                let resolved_type_name = format!("{}::{}", std::module_path!(), #reflected_type_name);
                let mut arguments = Vec::new();
                #generics_type_references_resolution_code;

                if schema.reserve_type(resolved_type_name.as_ref()) {
                    let mut reflected_type_def = #reflected_type_def;
                    reflected_type_def.__internal_rename_current(resolved_type_name.clone());

                    let mut unresolved_to_resolved_fields_type_refs = std::collections::HashMap::new();
                    #fields_type_references_resolution_code;
                    reflected_type_def.__internal_rebind_generic_parameters(&unresolved_to_resolved_fields_type_refs, schema);

                    schema.insert_type(reflected_type_def);
                }

                reflectapi::TypeReference::new(resolved_type_name, arguments)
            }
        }

        #[allow(unused_doc_comments)]
        impl #type_generics #type_ident #type_generics_idents_code #type_generics_where {
            fn #fn_reflectapi_ident() -> (reflectapi::TypeReference, reflectapi::Typespace) {
                let mut schema = reflectapi::Typespace::new();
                let resolved_type_ref = <Self as #trait_ident>::#fn_reflectapi_type_ident(&mut schema);
                schema.sort_types();
                (resolved_type_ref, schema)
            }
        }
    })
}

fn visit_type(cx: &Context, container: &ast::Container<'_>) -> Type {
    let (type_def_name, serde_name) =
        visit_name(cx, container.attrs.name(), Some(&container.original.ident));
    let type_def_description = parse_doc_attributes(&container.original.attrs);
    let attrs = parse_type_attributes(cx, &container.original.attrs);
    let codegen_config = reflectapi_schema::LanguageSpecificTypeCodegenConfig {
        rust: reflectapi_schema::RustTypeCodegenConfig {
            additional_derives: attrs
                .derives
                .iter()
                .map(|d| d.to_token_stream().to_string())
                .collect(),
        },
    };

    fn make_alias_type(
        type_def_name: String,
        type_def_description: String,
        serde_name: String,
        type_ref: reflectapi_schema::TypeReference,
        codegen_config: reflectapi_schema::LanguageSpecificTypeCodegenConfig,
    ) -> Struct {
        Struct {
            name: type_def_name,
            serde_name,
            description: type_def_description,
            parameters: Vec::new(),
            fields: Fields::Unnamed(vec![Field::new("0".into(), type_ref)]),
            transparent: true,
            codegen_config,
        }
    }

    match cx.reflectapi_type() {
        ReflectType::Input => {
            if let Some(input_type_attribute) = attrs.input_type {
                return make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, &input_type_attribute),
                    codegen_config,
                )
                .into();
            }
            if let Some(a) = container.attrs.type_from() {
                return make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, a),
                    codegen_config,
                )
                .into();
            }
            if let Some(a) = container.attrs.type_try_from() {
                return make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, a),
                    codegen_config,
                )
                .into();
            }
        }
        ReflectType::Output => {
            if let Some(output_type_attribute) = attrs.output_type {
                return make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, &output_type_attribute),
                    codegen_config,
                )
                .into();
            }
            if let Some(a) = container.attrs.type_into() {
                return make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, a),
                    codegen_config,
                )
                .into();
            }
        }
    }

    match &container.data {
        ast::Data::Enum(variants) => {
            let mut result = Enum::new(type_def_name);
            result.description = type_def_description;
            result.serde_name = serde_name;
            result.codegen_config = codegen_config;
            match container.attrs.tag() {
                serde_derive_internals::attr::TagType::External => {
                    result.representation = reflectapi_schema::Representation::External;
                }
                serde_derive_internals::attr::TagType::Internal { tag } => {
                    result.representation =
                        reflectapi_schema::Representation::Internal { tag: tag.clone() };
                }
                serde_derive_internals::attr::TagType::Adjacent { tag, content } => {
                    result.representation = reflectapi_schema::Representation::Adjacent {
                        tag: tag.clone(),
                        content: content.clone(),
                    };
                }
                serde_derive_internals::attr::TagType::None => {
                    result.representation = reflectapi_schema::Representation::None;
                }
            }
            for variant in variants {
                let variant_attrs = parse_variant_attributes(cx, &variant.original.attrs);

                if !match cx.reflectapi_type() {
                    ReflectType::Input => {
                        variant.attrs.skip_deserializing()
                            || variant.attrs.other()
                            || variant_attrs.input_skip
                    }
                    ReflectType::Output => {
                        variant.attrs.skip_serializing() || variant_attrs.output_skip
                    }
                } {
                    result
                        .variants
                        .push(visit_variant(cx, variant, attrs.discriminant));
                }
            }
            visit_generic_parameters(cx, container.generics, &mut result.parameters);
            result.into()
        }
        ast::Data::Struct(style, fields) => {
            if matches!(style, serde_derive_internals::ast::Style::Unit) {
                let unit_type: syn::Type = parse_quote! { () };
                let mut result = make_alias_type(
                    type_def_name,
                    type_def_description,
                    serde_name,
                    visit_field_type(cx, &unit_type),
                    codegen_config,
                );
                result.transparent = container.attrs.transparent();
                result.into()
            } else {
                let mut result = Struct::new(type_def_name);
                result.codegen_config = codegen_config;
                result.description = type_def_description;
                let fields = fields
                    .iter()
                    .filter_map(|field| visit_field(cx, field))
                    .collect::<Vec<_>>();

                result.fields = match style {
                    ast::Style::Struct => Fields::Named(fields),
                    ast::Style::Tuple | ast::Style::Newtype => Fields::Unnamed(fields),
                    ast::Style::Unit => Fields::None,
                };

                visit_generic_parameters(cx, container.generics, &mut result.parameters);
                result.transparent = container.attrs.transparent();
                result.into()
            }
        }
    }
}

fn visit_generic_parameters(
    cx: &Context,
    generics: &syn::Generics,
    parameters: &mut Vec<reflectapi_schema::TypeParameter>,
) {
    for param in generics.params.iter() {
        match param {
            syn::GenericParam::Type(type_param) => {
                let mut tp: TypeParameter = type_param.ident.to_string().into();
                tp.description = parse_doc_attributes(&type_param.attrs);
                parameters.push(tp);
                cx.encountered_generic_type(
                    type_param.ident.to_token_stream().to_string().into(),
                    type_param.ident.clone(),
                );
            }
            syn::GenericParam::Lifetime(lifetime_param) => {
                cx.impl_error(
                    lifetime_param,
                    format_args!(
                        "reflectapi::Input/reflectapi::Output does not support generic lifetime parameters"
                    ),
                );
            }
            syn::GenericParam::Const(const_param) => {
                cx.impl_error(
                    const_param,
                    format_args!(
                        "reflectapi::Input/reflectapi::Output does not support generic const parameters"
                    ),
                );
            }
        }
    }
}

fn visit_variant(
    cx: &Context,
    variant: &ast::Variant<'_>,
    use_discriminant: bool,
) -> reflectapi_schema::Variant {
    let (variant_def_name, serde_name) =
        visit_name(cx, variant.attrs.name(), Some(&variant.original.ident));

    let fields = variant
        .fields
        .iter()
        .filter_map(|field| visit_field(cx, field))
        .collect::<Vec<_>>();

    let mut discriminant = None;
    if use_discriminant {
        if let Some((_, d)) = variant.original.discriminant.as_ref() {
            discriminant = Some(
                // will be checked by compiler anyway
                d.to_token_stream().to_string().parse().unwrap_or_default(),
            );
        }
    }

    let fields = match variant.style {
        ast::Style::Struct => Fields::Named(fields),
        ast::Style::Tuple | ast::Style::Newtype => Fields::Unnamed(fields),
        ast::Style::Unit => Fields::None,
    };

    Variant {
        name: variant_def_name,
        serde_name,
        description: parse_doc_attributes(&variant.original.attrs),
        fields,
        discriminant,
        untagged: variant.attrs.untagged(),
    }
}

fn visit_field(cx: &Context, field: &ast::Field<'_>) -> Option<reflectapi_schema::Field> {
    let (field_name, serde_name) =
        visit_name(cx, field.attrs.name(), field.original.ident.as_ref());
    let attrs = parse_field_attributes(cx, &field.original.attrs);
    if match cx.reflectapi_type() {
        ReflectType::Input => attrs.input_skip || field.attrs.skip_deserializing(),
        ReflectType::Output => attrs.output_skip || field.attrs.skip_serializing(),
    } {
        return None;
    }
    let (field_type, field_transform) = match cx.reflectapi_type() {
        ReflectType::Input => (attrs.input_type, attrs.input_transform),
        ReflectType::Output => (attrs.output_type, attrs.output_transform),
    };

    let field_type = match field_type {
        Some(field_type) => visit_field_type(cx, &field_type),
        None => visit_field_type(cx, &field.original.ty),
    };

    let mut field_def = Field::new(field_name, field_type);
    field_def.deprecation_note = attrs.deprecation_note;
    field_def.transform_callback = field_transform;
    field_def.description = parse_doc_attributes(&field.original.attrs);
    field_def.serde_name = serde_name;
    field_def.required = match cx.reflectapi_type() {
        ReflectType::Input => field.attrs.default().is_none(),
        ReflectType::Output => field.attrs.skip_serializing_if().is_none(),
    };
    field_def.flattened = field.attrs.flatten();
    Some(field_def)
}

fn visit_field_type(cx: &Context, ty: &syn::Type) -> reflectapi_schema::TypeReference {
    let result: reflectapi_schema::TypeReference =
        naive_parse_as_type_reference(ty.to_token_stream().to_string().as_str());
    cx.encountered_field_type(result.clone(), ty.clone());
    result
}

fn visit_name<'a>(
    cx: &'a Context,
    name: &'a serde_derive_internals::attr::Name,
    ident: Option<&syn::Ident>,
) -> (String, String) {
    let result = match cx.reflectapi_type() {
        ReflectType::Input => name.deserialize_name(),
        ReflectType::Output => name.serialize_name(),
    };

    // check if normalized name contains invalid characters
    // and then use original ident name instead
    for (ind, c) in result.chars().enumerate() {
        // codegen tools should be able to handle camel case, kebab case and snake case names
        // for variants and fields
        if ident.is_some() && ind == 0 && !c.is_ascii_alphabetic() && c != '_'
            || !c.is_ascii_alphanumeric() && c != '_' && c != '-'
        {
            return (
                ident.map(|ident| ident.to_string()).unwrap_or("0".into()),
                result.into(),
            );
        }
    }

    // name is valid but it can be kebab case, handle this case automatically
    let normalized_result = result.replace('-', "_");
    if normalized_result != result {
        return (normalized_result, result.into());
    }
    (result.into(), String::new())
}
