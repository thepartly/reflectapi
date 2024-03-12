use quote::ToTokens;
use reflect_schema::*;

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

    TypeReference::new(name.trim().into(), parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naive_parse() {
        let t = naive_parse_as_type_reference("Vec<T>");
        assert_eq!(t, TypeReference::new("Vec".into(), vec!["T".into()]));
    }

    #[test]
    fn test_naive_parse_2() {
        let t = naive_parse_as_type_reference("Vec<Vec<T>>");
        assert_eq!(
            t,
            TypeReference::new(
                "Vec".into(),
                vec![TypeReference::new("Vec".into(), vec!["T".into()])]
            )
        );
    }

    #[test]
    fn test_naive_parse_3() {
        let t = naive_parse_as_type_reference("Vec<Vec<T>, Vec<T>>");
        assert_eq!(
            t,
            TypeReference::new(
                "Vec".into(),
                vec![
                    TypeReference::new("Vec".into(), vec!["T".into()]),
                    TypeReference::new("Vec".into(), vec!["T".into()])
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
                "Vec".into(),
                vec![
                    TypeReference::new("Vec".into(), vec!["T".into()]),
                    TypeReference::new("Vec".into(), vec!["T".into(), "U".into()])
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
                "Vec".into(),
                vec![
                    TypeReference::new("Vec".into(), vec!["T".into(), "U".into()]),
                    TypeReference::new("Vec".into(), vec!["T".into(), "U".into()])
                ]
            )
        );
    }
}

pub(crate) struct ParsedFieldAttributes {
    pub input_type: Option<syn::Type>,
    pub output_type: Option<syn::Type>,
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
pub(crate) fn parse_field_attributes(cx: &Context, field: &syn::Field) -> ParsedFieldAttributes {
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
                        result.output_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == INPUT_TYPE {
                // #[reflect(input_type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, INPUT_TYPE, &meta)? {
                    if cx.reflect_type() == ReflectType::Input {
                        result.input_type = Some(syn::Type::Path(syn::TypePath {
                            qself: path.qself,
                            path: path.path,
                        }));
                    }
                }
            } else if meta.path == TYPE {
                // #[reflect(type = "...")]
                if let Some(path) = parse_lit_into_expr_path(cx, TYPE, &meta)? {
                    let referred_type = syn::Type::Path(syn::TypePath {
                        qself: path.qself,
                        path: path.path,
                    });
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
                let path = meta.path.to_token_stream().to_string();
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
