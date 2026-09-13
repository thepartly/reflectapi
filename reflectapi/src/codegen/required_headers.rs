use std::collections::BTreeSet;

use anyhow::{bail, Context};
use check_keyword::CheckKeyword;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequiredHeader {
    pub name: String,
    pub ident: String,
}

pub(crate) fn resolve(names: &BTreeSet<String>) -> anyhow::Result<Vec<RequiredHeader>> {
    let mut resolved: Vec<RequiredHeader> = Vec::with_capacity(names.len());
    for name in names {
        let header = RequiredHeader {
            name: validate_name(name)?,
            ident: to_ident(name),
        };

        if let Some(clash) = resolved.iter().find(|other| other.ident == header.ident) {
            bail!(
                "required headers `{}` and `{}` both map to the identifier `{}` in generated code",
                clash.name,
                header.name,
                header.ident,
            );
        }

        resolved.push(header);
    }
    resolved.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(resolved)
}

fn validate_name(name: &str) -> anyhow::Result<String> {
    if name.is_empty() {
        bail!("required header name must not be empty");
    }

    let invalid = name.chars().find(|c| !is_token_char(*c));
    if let Some(invalid) = invalid {
        bail!("required header name `{name}` contains an invalid character: `{invalid}`");
    }

    Ok(name.to_ascii_lowercase())
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

fn to_ident(name: &str) -> String {
    let mut ident = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            ident.push(c.to_ascii_lowercase());
        } else {
            ident.push('_');
        }
    }

    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }

    ident
}

impl RequiredHeader {
    pub fn rust_ident(&self) -> String {
        self.ident.clone().into_safe()
    }

    pub fn python_ident(&self) -> String {
        if PYTHON_KEYWORDS.contains(&self.ident.as_str()) {
            format!("{}_", self.ident)
        } else {
            self.ident.clone()
        }
    }
}

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

pub(crate) fn resolve_for(
    language: &str,
    names: &BTreeSet<String>,
) -> anyhow::Result<Vec<RequiredHeader>> {
    resolve(names).context(format!("invalid `required_headers` for {language} codegen"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn lowercases_and_identifier_escapes() {
        let resolved = resolve(&set(&["X-Api-Key", "Authorization"])).unwrap();
        assert_eq!(
            resolved,
            vec![
                RequiredHeader {
                    name: "authorization".into(),
                    ident: "authorization".into(),
                },
                RequiredHeader {
                    name: "x-api-key".into(),
                    ident: "x_api_key".into(),
                },
            ]
        );
    }

    #[test]
    fn rejects_non_token_names() {
        let err = resolve(&set(&["x api key"])).unwrap_err().to_string();
        assert!(err.contains("invalid character"), "{err}");

        let err = resolve(&set(&[""])).unwrap_err().to_string();
        assert!(err.contains("must not be empty"), "{err}");
    }

    #[test]
    fn rejects_names_colliding_on_identifier() {
        let err = resolve(&set(&["x-api-key", "x.api.key"]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("x_api_key"), "{err}");
    }

    #[test]
    fn prefixes_leading_digit() {
        let resolved = resolve(&set(&["1st-header"])).unwrap();
        assert_eq!(resolved[0].ident, "_1st_header");
    }

    #[test]
    fn escapes_keywords_per_language() {
        let resolved = resolve(&set(&["type", "class"])).unwrap();
        let class = &resolved[0];
        let type_ = &resolved[1];
        assert_eq!(type_.rust_ident(), "r#type");
        assert_eq!(type_.python_ident(), "type");
        assert_eq!(class.python_ident(), "class_");
    }
}
