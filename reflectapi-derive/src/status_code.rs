//! `#[derive(StatusCode)]`: how a route's error enum is answered. Each variant
//! gives its status, `#[reflectapi(status = 404)]`, or takes the status and
//! headers of its one field, `#[reflectapi(transparent)]`. The statuses a
//! route's error must not give are refused as it compiles.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Data, DeriveInput, Fields, Variant};

use crate::symbol::{OUTPUT_SKIP, REFLECT, SKIP, STATUS, TRANSPARENT};

/// The one variant that may answer `500`.
const INTERNAL_ERROR: &str = "InternalError";

pub(crate) fn derive_status_code(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// What a variant answers.
enum Answer {
    Status(u16),
    Transparent,
}

/// The variant attributes this derive reads.
#[derive(Default)]
struct VariantAttributes {
    answer: Option<(Answer, Span)>,
    output_skip: bool,
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "`#[derive(reflectapi::StatusCode)]` is for an error enum; give a struct \
             its status with `impl reflectapi::StatusCode`",
        ));
    };
    let http = quote!(::reflectapi::__private::http);

    let mut errors: Option<syn::Error> = None;
    let mut push_error = |error: syn::Error| match &mut errors {
        Some(errors) => errors.combine(error),
        None => errors = Some(error),
    };
    let mut status_arms = Vec::new();
    let mut header_arms = Vec::new();
    let mut transparent_types = Vec::new();

    for variant in &data.variants {
        let ident = &variant.ident;
        let attributes = match variant_attributes(variant) {
            Ok(attributes) => attributes,
            Err(error) => {
                push_error(error);
                continue;
            }
        };
        if ident == INTERNAL_ERROR && !attributes.output_skip {
            push_error(syn::Error::new_spanned(
                ident,
                "`InternalError` must be `#[reflectapi(output_skip)]`: a client takes any \
                 5xx as the server's failure, not the route's error, so it has no place in \
                 the schema",
            ));
        }
        match attributes.answer {
            None => push_error(syn::Error::new_spanned(
                ident,
                format!(
                    "variant `{ident}` gives no status: add `#[reflectapi(status = 404)]`, or \
                     `#[reflectapi(transparent)]` to take its field's"
                ),
            )),
            Some((Answer::Status(status), span)) => {
                if let Err(error) = check_status(variant, status, span) {
                    push_error(error);
                    continue;
                }
                let pattern = match &variant.fields {
                    Fields::Named(_) => quote!(Self::#ident { .. }),
                    Fields::Unnamed(_) => quote!(Self::#ident(..)),
                    Fields::Unit => quote!(Self::#ident),
                };
                // `from_u16` fails only outside 100 to 999, which `check_status` refuses.
                status_arms.push(quote! {
                    #pattern => #http::StatusCode::from_u16(#status)
                        .unwrap_or(#http::StatusCode::INTERNAL_SERVER_ERROR)
                });
            }
            Some((Answer::Transparent, span)) => {
                if ident == INTERNAL_ERROR {
                    push_error(syn::Error::new(
                        span,
                        "`InternalError` answers 500: give it `status = 500`, not \
                         `transparent`",
                    ));
                    continue;
                }
                let (pattern, ty) = match &variant.fields {
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        (quote!(Self::#ident(inner)), &fields.unnamed[0].ty)
                    }
                    Fields::Named(fields) if fields.named.len() == 1 => {
                        let field = &fields.named[0].ident;
                        (quote!(Self::#ident { #field: inner }), &fields.named[0].ty)
                    }
                    _ => {
                        push_error(syn::Error::new(
                            span,
                            format!(
                                "a `transparent` variant takes its one field's status and \
                                 headers: `{ident}` must have exactly one field"
                            ),
                        ));
                        continue;
                    }
                };
                status_arms.push(quote!(#pattern => ::reflectapi::StatusCode::status_code(inner)));
                header_arms
                    .push(quote!(#pattern => ::reflectapi::StatusCode::response_headers(inner)));
                transparent_types.push(ty.clone());
            }
        }
    }
    if let Some(errors) = errors {
        return Err(errors);
    }

    let name = &input.ident;
    let mut generics = input.generics.clone();
    if !generics.params.is_empty() {
        let where_clause = generics.make_where_clause();
        for ty in &transparent_types {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ::reflectapi::StatusCode));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let status_code = if data.variants.is_empty() {
        quote!(match *self {})
    } else {
        quote!(match self { #(#status_arms,)* })
    };
    let response_headers = if header_arms.is_empty() {
        // The trait's default: no headers.
        quote!()
    } else {
        let rest = (header_arms.len() < data.variants.len())
            .then(|| quote!(_ => #http::HeaderMap::new(),));
        quote! {
            fn response_headers(&self) -> #http::HeaderMap {
                match self {
                    #(#header_arms,)*
                    #rest
                }
            }
        }
    };

    Ok(quote! {
        impl #impl_generics ::reflectapi::StatusCode for #name #ty_generics #where_clause {
            fn status_code(&self) -> #http::StatusCode {
                #status_code
            }

            #response_headers
        }
    })
}

/// A variant's `status` or `transparent`, and whether it is out of the
/// output schema. Other `#[reflectapi(...)]` keys belong to the `Input` and
/// `Output` derives, which check them, and are passed over.
fn variant_attributes(variant: &Variant) -> syn::Result<VariantAttributes> {
    let mut attributes = VariantAttributes::default();
    for attr in &variant.attrs {
        if attr.path() != REFLECT {
            continue;
        }
        if let syn::Meta::List(list) = &attr.meta {
            if list.tokens.is_empty() {
                continue;
            }
        }
        attr.parse_nested_meta(|meta| {
            let answer = if meta.path == STATUS {
                let status: syn::LitInt = meta.value()?.parse()?;
                let span = status.span();
                let status = status
                    .base10_parse::<u16>()
                    .map_err(|_| syn::Error::new(span, "a status is a number from 100 to 599"))?;
                Some((Answer::Status(status), span))
            } else if meta.path == TRANSPARENT {
                Some((Answer::Transparent, meta.path.span()))
            } else {
                if meta.path == OUTPUT_SKIP || meta.path == SKIP {
                    attributes.output_skip = true;
                }
                pass_over(&meta)?;
                None
            };
            if let Some(answer) = answer {
                if attributes.answer.is_some() {
                    return Err(meta.error(format!(
                        "variant `{}` gives its status twice: give one of `status = …` and \
                         `transparent`",
                        variant.ident
                    )));
                }
                attributes.answer = Some(answer);
            }
            Ok(())
        })?;
    }
    Ok(attributes)
}

/// Reads past a key this derive does not use: `key`, `key = value` or
/// `key(...)`.
fn pass_over(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|nested| pass_over(&nested))?;
    }
    Ok(())
}

/// Refuses a status a route's error must not give.
fn check_status(variant: &Variant, status: u16, span: Span) -> syn::Result<()> {
    let refused = |why: &str| Err(syn::Error::new(span, format!("status {status}: {why}")));
    match status {
        100..=399 => refused(
            "a client takes a 1xx, 2xx or 3xx for an answer, or acts on it (follows a \
             redirect, caches a 301); a route's error gives a 4xx, or 500",
        ),
        400 | 405 | 415 => refused(
            "reflectapi answers it itself, for a body or headers it can't decode, a \
             method a route doesn't take, or a content type it doesn't speak; a route's \
             error gives another 4xx, such as 422",
        ),
        408 | 413 | 414 | 431 => {
            refused("the transport's or the server's, for a request a route never sees")
        }
        500 if variant.ident != INTERNAL_ERROR => {
            refused("only a variant named `InternalError` answers 500, the server's failure")
        }
        500 => Ok(()),
        501..=599 => refused(
            "a gateway's or the server's own, for a request it can't take or finish; a \
             route's error gives a 4xx, or 500 on `InternalError`",
        ),
        401..=499 => Ok(()),
        _ => refused("not an HTTP status a response can give: one from 100 to 599"),
    }
}
