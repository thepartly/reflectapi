use proc_macro::TokenStream;
use quote::ToTokens;
use reflect_schema::{Enum, Field, Struct, Type, TypeParameter};
use syn::parse_quote;

use crate::{
    context::{Context, ReflectType},
    parser::{naive_parse_as_type_reference, parse_doc_attributes, parse_field_attributes},
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

    let type_generics_idents = context_encounters
        .generics
        .iter()
        .map(|(_, ident)| ident)
        .collect::<Vec<_>>();
    let type_genercis_idents_code = if type_generics_idents.is_empty() {
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
                let mut resolved_type_ref = <#origin_field_syn_type as #trait_ident>::#fn_reflect_type_ident(schema);
                let unresolved_field_type_ref = #unresolved_field_type_ref;
                unresolved_to_resolved_fields_type_refs.insert(#unresolved_field_type_ref, resolved_type_ref);
            }
        });
    }

    let mut generics_type_references_resolution_code = quote::quote!();
    for (_, origin_type_param) in context_encounters.generics.into_iter() {
        generics_type_references_resolution_code.extend(quote::quote! {
            {
                parameters.push(<#origin_type_param as #trait_ident>::#fn_reflect_type_ident(schema));
            }
        });
    }

    let reflected_type_def = crate::tokenizable_schema::TokenizableType::new(&reflected_type_def);
    TokenStream::from(quote::quote! {
        #[allow(unused_doc_comments)]
        impl #type_generics #trait_ident for #type_ident #type_genercis_idents_code #type_generics_where {
            fn #fn_reflect_type_ident(schema: &mut reflect::Typespace) -> reflect::TypeReference {
                let resolved_type_name = format!("{}::{}", std::module_path!(), #reflected_type_name);
                let mut parameters = Vec::new();
                #generics_type_references_resolution_code;

                if schema.reserve_type(resolved_type_name.as_ref()) {
                    let mut reflected_type_def = #reflected_type_def;
                    reflected_type_def.__internal_rename_current(resolved_type_name.clone());

                    let mut unresolved_to_resolved_fields_type_refs = std::collections::HashMap::new();
                    #fields_type_references_resolution_code;
                    reflected_type_def.__internal_rebind_generic_parameters(&unresolved_to_resolved_fields_type_refs, schema);

                    schema.insert_type(reflected_type_def);
                }

                reflect::TypeReference::new(resolved_type_name, parameters)
            }
        }

        #[allow(unused_doc_comments)]
        impl #type_generics #type_ident #type_genercis_idents_code #type_generics_where {
            fn #fn_reflect_ident() -> (reflect::TypeReference, reflect::Typespace) {
                let mut schema = reflect::Typespace::new();
                let resolved_type_ref = <Self as #trait_ident>::#fn_reflect_type_ident(&mut schema);
                schema.sort_types();
                (resolved_type_ref, schema)
            }
        }
    })
}

fn visit_type<'a>(cx: &Context, container: &serde_derive_internals::ast::Container<'a>) -> Type {
    let type_def_name = visit_name(cx, container.attrs.name()).into();
    let type_def: Type = match &container.data {
        serde_derive_internals::ast::Data::Enum(variants) => {
            let mut result = Enum::new(type_def_name);
            result.description = parse_doc_attributes(&container.original.attrs);
            match container.attrs.tag() {
                serde_derive_internals::attr::TagType::External => {
                    result.representation = reflect_schema::Representation::External;
                }
                serde_derive_internals::attr::TagType::Internal { tag } => {
                    result.representation =
                        reflect_schema::Representation::Internal { tag: tag.clone() };
                }
                serde_derive_internals::attr::TagType::Adjacent { tag, content } => {
                    result.representation = reflect_schema::Representation::Adjacent {
                        tag: tag.clone(),
                        content: content.clone(),
                    };
                }
                serde_derive_internals::attr::TagType::None => {
                    result.representation = reflect_schema::Representation::None;
                }
            }
            for variant in variants {
                if !match cx.reflect_type() {
                    ReflectType::Input => {
                        variant.attrs.skip_deserializing() || variant.attrs.other()
                    }
                    ReflectType::Output => variant.attrs.skip_serializing(),
                } {
                    result.variants.push(visit_variant(cx, variant));
                }
            }
            visit_generic_parameters(cx, &container.generics, &mut result.parameters);
            result.into()
        }
        serde_derive_internals::ast::Data::Struct(style, fields) => {
            if matches!(style, serde_derive_internals::ast::Style::Unit) {
                let mut result = Struct::new(type_def_name);
                result.description = parse_doc_attributes(&container.original.attrs);
                // there should be no fields on unit structs
                // but we expose it as a newtype struct with a single Unit type field
                result.fields.push(Field::new("0".into(), "()".into()));
                visit_generic_parameters(cx, &container.generics, &mut result.parameters);
                result.transparent = container.attrs.transparent();
                let unit_type: syn::Type = parse_quote! { () };
                visit_field_type(cx, &unit_type);
                result.into()
            } else {
                let mut result = Struct::new(type_def_name);
                result.description = parse_doc_attributes(&container.original.attrs);
                for field in fields {
                    if !match cx.reflect_type() {
                        ReflectType::Input => field.attrs.skip_deserializing(),
                        ReflectType::Output => field.attrs.skip_serializing(),
                    } {
                        result.fields.push(visit_field(cx, field));
                    }
                }
                visit_generic_parameters(cx, &container.generics, &mut result.parameters);
                result.transparent = container.attrs.transparent();
                result.into()
            }
        }
    };
    type_def
}

fn visit_generic_parameters<'a>(
    cx: &Context,
    generics: &syn::Generics,
    parameters: &mut Vec<reflect_schema::TypeParameter>,
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
                        "reflect::Input/reflect::Output does not support generic lifetime parameters"
                    ),
                );
            }
            syn::GenericParam::Const(const_param) => {
                cx.impl_error(
                    const_param,
                    format_args!(
                        "reflect::Input/reflect::Output does not support generic const parameters"
                    ),
                );
            }
        }
    }
}

fn visit_variant<'a>(
    cx: &Context,
    variant: &serde_derive_internals::ast::Variant<'a>,
) -> reflect_schema::Variant {
    let variant_def_name = visit_name(cx, variant.attrs.name()).into();
    let mut result = reflect_schema::Variant::new(variant_def_name);
    result.description = parse_doc_attributes(&variant.original.attrs);
    if let Some(discriminant) = variant.original.discriminant.as_ref() {
        result.discriminant = Some(
            discriminant
                .1
                .to_token_stream()
                .to_string()
                .parse()
                .unwrap_or_default(), // will be checked by compiler anyway
        );
    }
    for field in &variant.fields {
        if !match cx.reflect_type() {
            ReflectType::Input => field.attrs.skip_deserializing(),
            ReflectType::Output => field.attrs.skip_serializing(),
        } {
            result.fields.push(visit_field(cx, field));
        }
    }
    result.untagged = variant.attrs.untagged();
    result
}

fn visit_field<'a>(
    cx: &Context,
    field: &serde_derive_internals::ast::Field<'a>,
) -> reflect_schema::Field {
    let field_name = visit_name(cx, field.attrs.name()).into();
    let attrs = parse_field_attributes(cx, field.original);
    let (field_type, field_transform) = match cx.reflect_type() {
        ReflectType::Input => (attrs.input_type, attrs.input_transform),
        ReflectType::Output => (attrs.output_type, attrs.output_transform),
    };

    let field_type = match field_type {
        Some(field_type) => visit_field_type(cx, &field_type),
        None => visit_field_type(cx, &field.original.ty),
    };

    let mut field_def = Field::new(field_name, field_type);
    field_def.transform_callback = field_transform;
    field_def.description = parse_doc_attributes(&field.original.attrs);
    field_def.required = match cx.reflect_type() {
        ReflectType::Input => field.attrs.default().is_none(),
        ReflectType::Output => field.attrs.skip_serializing_if().is_none(),
    };
    field_def.flattened = field.attrs.flatten();
    field_def
}

fn visit_field_type<'a>(cx: &Context, ty: &syn::Type) -> reflect_schema::TypeReference {
    let result: reflect_schema::TypeReference =
        naive_parse_as_type_reference(ty.to_token_stream().to_string().as_str());
    cx.encountered_field_type(result.clone(), ty.clone());
    result
}

fn visit_name<'a>(cx: &'a Context, name: &'a serde_derive_internals::attr::Name) -> &'a str {
    match cx.reflect_type() {
        ReflectType::Input => name.deserialize_name(),
        ReflectType::Output => name.serialize_name(),
    }
}
