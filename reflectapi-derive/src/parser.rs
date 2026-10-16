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
    for (i, c) in s.char_indices() {
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
    pub open: bool,
    pub derives: Vec<syn::Meta>,
}

/// `#[deprecated]` plus `#[reflectapi(deprecated_since = "..", sunset = "..")]`
/// on a field or variant. Dates are kept as literals so later checks can
/// point at them.
#[derive(Debug, Default)]
pub(crate) struct ParsedDeprecation {
    pub note: Option<String>,
    /// `since` of `#[deprecated]`, a release such as a crate version.
    pub release: Option<String>,
    pub since: Option<syn::LitStr>,
    pub sunset: Option<syn::LitStr>,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedFieldAttributes {
    pub deprecation: ParsedDeprecation,
    pub input_type: Option<syn::Type>,
    pub output_type: Option<syn::Type>,
    pub input_transform: String,
    pub output_transform: String,
    pub input_skip: bool,
    pub output_skip: bool,
    pub header: bool,
    pub hidden: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedVariantAttributes {
    pub deprecation: ParsedDeprecation,
    pub input_skip: bool,
    pub output_skip: bool,
}

pub(crate) fn parse_doc_attributes(attrs: &Vec<syn::Attribute>) -> String {
    let mut result = Vec::new();
    for attr in attrs {
        if attr.path() != DOC {
            continue;
        }

        // Read the actual string value of the `#[doc = "..."]` literal.
        // The previous implementation went through `to_token_stream()`,
        // which produced the *Rust source* representation (with the
        // surrounding quotes and any embedded `\"` escapes still in
        // place), so a doc comment containing literal double quotes
        // landed in the schema as `\"`-prefixed bytes.
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(litstr),
                ..
            }) = &meta.value
            {
                result.push(litstr.value().trim().to_string());
            }
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
            } else if meta.path.is_ident("open") {
                // #[reflectapi(open)] applies only to generated response enums.
                result.open = true;
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

/// Parses `#[deprecated]`, `#[deprecated = ".."]` and
/// `#[deprecated(since = "..", note = "..")]` into the note and the `since`
/// literal.
fn parse_deprecated_attr(attr: &syn::Attribute) -> syn::Result<(String, Option<syn::LitStr>)> {
    let expect_str = |expr: &syn::Expr| match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(s.value()),
        _ => Err(syn::Error::new_spanned(
            expr,
            "expected a string literal for the deprecation note",
        )),
    };
    match &attr.meta {
        syn::Meta::Path(_) => Ok((String::new(), None)),
        syn::Meta::NameValue(nv) => Ok((expect_str(&nv.value)?, None)),
        syn::Meta::List(list) => {
            let mut note = String::new();
            let mut since = None;
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("note") {
                    note = meta.value()?.parse::<syn::LitStr>()?.value();
                } else if meta.path.is_ident("since") {
                    since = Some(meta.value()?.parse::<syn::LitStr>()?);
                } else if meta.input.peek(syn::Token![=]) {
                    // rustc reports unknown items itself; just skip the value.
                    meta.value()?.parse::<syn::Expr>()?;
                }
                Ok(())
            })?;
            Ok((note, since))
        }
    }
}

/// Reads the `#[deprecated]` attribute into `deprecation`.
///
/// rustc accepts any string as `since`, so it is kept verbatim as the
/// release the item was deprecated in, never as a date.
fn parse_deprecation(cx: &Context, attr: &syn::Attribute, deprecation: &mut ParsedDeprecation) {
    let (note, since) = match parse_deprecated_attr(attr) {
        Ok(parsed) => parsed,
        Err(err) => return cx.syn_error(err),
    };
    deprecation.note = Some(note);
    deprecation.release = since.map(|lit| lit.value());
}

/// Parses `name = "YYYY-MM-DD"` inside `#[reflectapi(..)]` into `slot`.
fn parse_date(
    cx: &Context,
    name: Symbol,
    meta: &syn::meta::ParseNestedMeta<'_>,
    slot: &mut Option<syn::LitStr>,
) -> syn::Result<()> {
    if let Some(lit) = parse_lit_str(cx, name, name, meta)? {
        set_date(cx, &name.to_string(), slot, lit);
    }
    Ok(())
}

fn set_date(cx: &Context, name: &str, slot: &mut Option<syn::LitStr>, lit: syn::LitStr) {
    if let Err(err) = reflectapi_schema::validate_deprecation_dates(Some(&lit.value()), None) {
        return cx.impl_error(&lit, format!("`{name}`: {err}"));
    }
    match slot {
        Some(existing) if existing.value() != lit.value() => cx.impl_error(
            &lit,
            format!(
                "conflicting deprecation dates `{}` and `{}`",
                existing.value(),
                lit.value()
            ),
        ),
        _ => *slot = Some(lit),
    }
}

/// Checks the parts of a deprecation against each other once all attributes
/// of the item have been read.
fn check_deprecation(cx: &Context, deprecation: &ParsedDeprecation) {
    if deprecation.note.is_none() {
        for date in deprecation.since.iter().chain(&deprecation.sunset) {
            cx.impl_error(
                date,
                "deprecation dates require `#[deprecated]` on the same item",
            );
        }
        return;
    }
    let (Some(since), Some(sunset)) = (&deprecation.since, &deprecation.sunset) else {
        return;
    };
    if let Err(err) =
        reflectapi_schema::validate_deprecation_dates(Some(&since.value()), Some(&sunset.value()))
    {
        cx.impl_error(sunset, err);
    }
}

impl ParsedDeprecation {
    /// The note, release, deprecation date and sunset, in that order.
    pub fn into_parts(
        self,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            self.note,
            self.release,
            self.since.map(|lit| lit.value()),
            self.sunset.map(|lit| lit.value()),
        )
    }
}

pub(crate) fn parse_variant_attributes(
    cx: &Context,
    attributes: &[syn::Attribute],
) -> ParsedVariantAttributes {
    let mut deprecation = ParsedDeprecation::default();
    let mut input_skip = false;
    let mut output_skip = false;

    for attr in attributes.iter() {
        if attr.path() == DEPRECATED {
            parse_deprecation(cx, attr, &mut deprecation);
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
            } else if meta.path == DEPRECATED_SINCE {
                // #[reflectapi(deprecated_since = "...")]
                parse_date(cx, DEPRECATED_SINCE, &meta, &mut deprecation.since)?;
            } else if meta.path == SUNSET {
                // #[reflectapi(sunset = "...")]
                parse_date(cx, SUNSET, &meta, &mut deprecation.sunset)?;
            } else {
                let path = meta.path.to_token_stream().to_string();
                return Err(meta.error(format_args!("unknown reflect variant attribute `{path}`")));
            }
            Ok(())
        }) {
            cx.syn_error(err);
        }
    }

    check_deprecation(cx, &deprecation);

    ParsedVariantAttributes {
        deprecation,
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
            parse_deprecation(cx, attr, &mut result.deprecation);
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
            } else if meta.path == HEADER {
                // #[reflectapi(header)]
                result.header = true;
            } else if meta.path == HIDDEN {
                // #[reflectapi(hidden)]
                result.hidden = true;
            } else if meta.path == DEPRECATED_SINCE {
                // #[reflectapi(deprecated_since = "...")]
                parse_date(cx, DEPRECATED_SINCE, &meta, &mut result.deprecation.since)?;
            } else if meta.path == SUNSET {
                // #[reflectapi(sunset = "...")]
                parse_date(cx, SUNSET, &meta, &mut result.deprecation.sunset)?;
            } else {
                let path = meta.path.to_token_stream().to_string();
                return Err(meta.error(format_args!("unknown reflect field attribute `{path}`")));
            }
            Ok(())
        }) {
            cx.syn_error(err);
        }
    }
    check_deprecation(cx, &result.deprecation);
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
                format!("unexpected suffix `{suffix}` on string literal"),
            );
        }
        Ok(Some(lit.clone()))
    } else {
        cx.impl_error(
            expr,
            format!(
                "expected reflect {attr_name} attribute to be a string: `{meta_item_name} = \"...\"`"
            ),
        );
        Ok(None)
    }
}
