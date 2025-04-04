use quote::ToTokens;
use reflectapi_schema::*;
use syn::parse::Parse;

use crate::{
    context::{Context, ReflectType},
    symbol::{Symbol, *},
};

pub(crate) fn naive_parse_as_type_reference(s: &str) -> TypeReference {
    // split generics by comma excluding commas inside of nested < >
    let mut name = s;
    let mut parameters = Vec::new();

    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '<' => {
                if depth == 0 {
                    name = &s[start..i];
                    start = i + 1;
                }
                depth += 1;
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    if s[start..i].chars().all(|i| i.is_whitespace()) {
                        start = i + 1;
                        continue;
                    }
                    parameters.push(naive_parse_as_type_reference(&s[start..i]));
                    start = i + 1;
                }
            }
            ',' if depth == 1 => {
                if s[start..i].chars().all(|i| i.is_whitespace()) {
                    start = i + 1;
                    continue;
                }
                parameters.push(naive_parse_as_type_reference(&s[start..i]));
                start = i + 1;
            }
            _ => {}
        }
    }

    TypeReference::new(name.trim(), parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naive_parse() {
        let t = naive_parse_as_type_reference("Vec<T>");
        assert_eq!(t, TypeReference::new("Vec", vec!["T".into()]));
    }

    #[test]
    fn test_naive_parse_2() {
        let t = naive_parse_as_type_reference("Vec<Vec<T>>");
        assert_eq!(
            t,
            TypeReference::new("Vec", vec![TypeReference::new("Vec", vec!["T".into()])])
        );
    }

    #[test]
    fn test_naive_parse_3() {
        let t = naive_parse_as_type_reference("Vec<Vec<T>, Vec<T>>");
        assert_eq!(
            t,
            TypeReference::new(
                "Vec",
                vec![
                    TypeReference::new("Vec", vec!["T".into()]),
                    TypeReference::new("Vec", vec!["T".into()])
                ]
            )
        );
    }

    #[test]
    fn test_naive_parse_4() {
        let t = naive_parse_as_type_reference("Vec<Vec<T>, Vec<T, U>>");
        assert_eq!(
            t,
            TypeReference::new(
                "Vec",
                vec![
                    TypeReference::new("Vec", vec!["T".into()]),
                    TypeReference::new("Vec", vec!["T".into(), "U".into()])
                ]
            )
        );
    }

    #[test]
    fn test_naive_parse_5() {
        let t = naive_parse_as_type_reference("Vec<Vec<T, U>, Vec<T, U>>");
        assert_eq!(
            t,
            TypeReference::new(
                "Vec",
                vec![
                    TypeReference::new("Vec", vec!["T".into(), "U".into()]),
                    TypeReference::new("Vec", vec!["T".into(), "U".into()])
                ]
            )
        );
    }
}

#[derive(Default)]
pub(crate) struct ParsedTypeAttributes {
    pub input_type: Option<syn::Type>,
    pub output_type: Option<syn::Type>,
    pub discriminant: bool,
    pub derives: Vec<syn::Meta>,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedFieldAttributes {
    pub deprecation_note: Option<String>,
    pub input_type: Option<syn::Type>,
    pub output_type: Option<syn::Type>,
    pub input_transform: String,
    pub output_transform: String,
    pub input_skip: bool,
    pub output_skip: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedVariantAttributes {
    pub input_skip: bool,
    pub output_skip: bool,
}

pub(crate) fn parse_doc_attributes(attrs: &Vec<syn::Attribute>) -> String {
    let mut result = Vec::new();
    for attr in attrs {
        if attr.path() != DOC {
            continue;
        }

        if let syn::Meta::NameValue(meta) = &attr.meta {
            result.push(
                meta.value
                    .to_token_stream()
                    .to_string()
                    .as_str()
                    .trim_matches('"')
                    .trim()
                    .to_string(),
            );
        }
    }
    result.join("\n")
}

/// Extract out the `#[reflectapi(...)]` attributes from a type definition.
pub(crate) fn parse_type_attributes(
    cx: &Context,
    attributes: &[syn::Attribute],
) -> ParsedTypeAttributes {
    let mut result = ParsedTypeAttributes::default();

    for attr in attributes.iter() {
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
                // #[reflectapi(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Output {
                        result.output_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == INPUT_TYPE {
                // #[reflectapi(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Input {
                        result.input_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == TYPE {
                // #[reflectapi(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    let referred_type = syn::Type::Path(syn::TypePath {
                        qself: path.qself,
                        path: path.path,
                    });
                    result.output_type = Some(referred_type.clone());
                    result.input_type = Some(referred_type);
                }
            } else if meta.path == DISCRIMINANT {
                // #[reflectapi(discriminant)]
                result.discriminant = true;
            } else if meta.path == DERIVE {
                // #[reflectapi(derive(...))]
                let metas;
                syn::parenthesized!(metas in meta.input);
                result
                    .derives
                    .extend(metas.parse_terminated(syn::Meta::parse, syn::Token![,])?);
            } else {
                let path = meta.path.to_token_stream().to_string();
                return Err(meta.error(format_args!("unknown reflect type attribute `{path}`")));
            }
            Ok(())
        }) {
            cx.syn_error(err);
        }
    }
    result
}

fn parse_deprecated_attr(attr: &syn::Attribute) -> syn::Result<String> {
    assert!(attr.path().is_ident("deprecated"));
    match &attr.meta {
        syn::Meta::Path(_) => Ok("".to_string()),
        syn::Meta::NameValue(nv) => match &nv.value {
            syn::Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Str(s) => Ok(s.value()),
                _ => panic!("expected string literal for deprecation note"),
            },
            _ => panic!("expected string literal for deprecation note"),
        },
        syn::Meta::List(list) => {
            let mut s = String::new();
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("note") {
                    s = meta.value()?.parse::<syn::LitStr>()?.value();
                }
                Ok(())
            })?;
            Ok(s)
        }
    }
}

pub(crate) fn parse_variant_attributes(
    cx: &Context,
    attributes: &[syn::Attribute],
) -> ParsedVariantAttributes {
    let mut input_skip = false;
    let mut output_skip = false;

    for attr in attributes.iter() {
        if attr.path() != REFLECT {
            continue;
        }

        if let syn::Meta::List(meta) = &attr.meta {
            if meta.tokens.is_empty() {
                continue;
            }
        }

        if let Err(err) = attr.parse_nested_meta(|meta| {
            if meta.path == INPUT_SKIP {
                // #[reflectapi(input_skip)]
                input_skip = true;
            } else if meta.path == OUTPUT_SKIP {
                // #[reflectapi(output_skip)]
                output_skip = true;
            } else if meta.path == SKIP {
                // #[reflectapi(skip)]
                input_skip = true;
                output_skip = true;
            } else {
                let path = meta.path.to_token_stream().to_string();
                return Err(meta.error(format_args!("unknown reflect variant attribute `{path}`")));
            }
            Ok(())
        }) {
            cx.syn_error(err);
        }
    }

    ParsedVariantAttributes {
        input_skip,
        output_skip,
    }
}

pub(crate) fn parse_field_attributes(
    cx: &Context,
    attributes: &[syn::Attribute],
) -> ParsedFieldAttributes {
    let mut result = ParsedFieldAttributes::default();

    for attr in attributes.iter() {
        if attr.path() == DEPRECATED {
            match parse_deprecated_attr(attr) {
                Ok(note) => result.deprecation_note = Some(note),
                Err(err) => cx.syn_error(err),
            }
        }

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
                // #[reflectapi(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Output {
                        result.output_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == INPUT_TYPE {
                // #[reflectapi(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Input {
                        result.input_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == TYPE {
                // #[reflectapi(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    let referred_type = syn::Type::Path(syn::TypePath {
                        qself: path.qself,
                        path: path.path,
                    });
                    result.output_type = Some(referred_type.clone());
                    result.input_type = Some(referred_type);
                }
            } else if meta.path == OUTPUT_TRANSFORM {
                // #[reflectapi(output_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, OUTPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Output {
                        result.output_transform = path.to_token_stream().to_string();
                    }
                }
            } else if meta.path == INPUT_TRANSFORM {
                // #[reflectapi(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflectapi_type() == ReflectType::Input {
                        result.input_transform = path.to_token_stream().to_string();
                    }
                }
            } else if meta.path == TRANSFORM {
                // #[reflectapi(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    result.output_transform = path.to_token_stream().to_string();
                    result.input_transform = path.to_token_stream().to_string();
                }
            } else if meta.path == INPUT_SKIP {
                // #[reflectapi(input_skip)]
                result.input_skip = true;
            } else if meta.path == OUTPUT_SKIP {
                // #[reflectapi(output_skip)]
                result.output_skip = true;
            } else if meta.path == SKIP {
                // #[reflectapi(skip)]
                result.input_skip = true;
                result.output_skip = true;
            } else {
                let path = meta.path.to_token_stream().to_string();
                return Err(meta.error(format_args!("unknown reflect field attribute `{path}`")));
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
    meta: &syn::meta::ParseNestedMeta<'_>,
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
    meta: &syn::meta::ParseNestedMeta<'_>,
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
                "expected reflect {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Ok(None)
    }
}
