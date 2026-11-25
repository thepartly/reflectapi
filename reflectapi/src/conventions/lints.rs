//! Lints: checks of a schema against the rules of the
//! [API design standard](https://reflectapi.rs/design/standard.html) that
//! its shape can show.
//!
//! Each lint is a function for `Builder::validate`, and checks one rule. An
//! error it reports names the rule in [`ValidationError::rule`] and links to
//! it in its message.
//!
//! ```rust
//! # #[derive(serde::Deserialize, reflectapi::Input)]
//! # struct PetsListRequest {}
//! # #[derive(serde::Serialize, reflectapi::Output)]
//! # struct PetsListResponse { items: Vec<String> }
//! # async fn pets_list(_: (), _: PetsListRequest, _: reflectapi::Empty) -> PetsListResponse {
//! #     PetsListResponse { items: vec![] }
//! # }
//! use reflectapi::conventions::lints;
//!
//! let mut builder = reflectapi::Builder::<()>::new()
//!     .route(pets_list, |b| b.name("pets.list"));
//! // Every lint:
//! for lint in lints::ALL {
//!     builder = builder.validate(*lint);
//! }
//! builder.build().unwrap();
//! ```
//!
//! To pick some, pass each to `validate`, such as
//! `.validate(lints::call_names_kebab_case)`. To relax one for some calls,
//! wrap it in a function of your own that filters its errors.

use std::collections::{HashMap, HashSet};

use crate::{
    Fields, Function, OutputType, Representation, Schema, Type, TypeReference, Typespace,
    ValidationError, ValidationPointer,
};

/// A lint, for `Builder::validate`.
pub type Lint = fn(&Schema) -> Vec<ValidationError>;

/// Every lint of this module.
pub const ALL: &[Lint] = &[
    call_name_form,
    call_names_kebab_case,
    call_resource_levels,
    call_version_suffix,
    call_actions_standard,
    call_actions_not_parameters,
    request_is_object,
    response_is_object,
    names_snake_case,
    enums_kind_tag,
    header_names_kebab_case,
    header_names_no_x_prefix,
    call_type_names,
    type_names_no_kind_suffix,
    type_path_depth,
    type_modules_not_implementation,
    error_cases_no_error_suffix,
];

/// Runs every lint of this module.
pub fn all(schema: &Schema) -> Vec<ValidationError> {
    ALL.iter().flat_map(|lint| lint(schema)).collect()
}

const STANDARD: &str = "https://reflectapi.rs/design/standard.html";

fn error(pointer: ValidationPointer, rule: &str, message: String) -> ValidationError {
    ValidationError::new(pointer, format!("{message}: see {STANDARD}#{rule}")).with_rule(rule)
}

fn function_error(f: &Function, rule: &str, message: String) -> ValidationError {
    error(ValidationPointer::Function(f.name().into()), rule, message)
}

// Call names

/// A call's action, without the suffixes that version it (`-v2`) or mark it
/// experimental (`-experimental`).
fn base_action(name: &str) -> &str {
    let mut action = name.rsplit('.').next().unwrap_or(name);
    loop {
        if let Some(rest) = action.strip_suffix("-experimental") {
            action = rest;
            continue;
        }
        if let Some(at) = action.rfind("-v") {
            let digits = &action[at + 2..];
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                action = &action[..at];
                continue;
            }
        }
        return action;
    }
}

/// A call name has a resource and an action, and no empty part:
/// `[api-call-name-form]`.
pub fn call_name_form(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter(|f| {
            let parts: Vec<_> = f.name().split('.').collect();
            parts.len() < 2 || parts.iter().any(|part| part.is_empty())
        })
        .map(|f| {
            function_error(
                f,
                "api-call-name-form",
                format!(
                    "`{}` isn't of the form `[category.]resources[.nested-resources].action`",
                    f.name()
                ),
            )
        })
        .collect()
}

fn is_kebab_case(part: &str) -> bool {
    !part.is_empty()
        && part.split('-').all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Each part of a call name is in lower-case `kebab-case`:
/// `[api-call-name-kebab-case]`.
pub fn call_names_kebab_case(schema: &Schema) -> Vec<ValidationError> {
    let mut errors = vec![];
    for f in schema.functions() {
        for part in f.name().split('.').filter(|part| !part.is_empty()) {
            if !is_kebab_case(part) {
                errors.push(function_error(
                    f,
                    "api-call-name-kebab-case",
                    format!("`{part}` isn't in lower-case kebab-case"),
                ));
            }
        }
    }
    errors
}

/// A call name has at most two levels of resources, after an optional
/// category: `[api-resource-two-levels]`. A version written as a part of its
/// own, which [`call_version_suffix`] reports, isn't counted.
pub fn call_resource_levels(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter(|f| {
            f.name()
                .split('.')
                .filter(|part| !is_version_word(part))
                .count()
                > 4
        })
        .map(|f| {
            function_error(
                f,
                "api-resource-two-levels",
                format!(
                    "`{}` has more than two levels of resources after a category",
                    f.name()
                ),
            )
        })
        .collect()
}

/// A call's version is a suffix of its action, `users.invite-v2`, not a part
/// of its own, `users.v2.invite`: `[api-call-version-suffix]`.
pub fn call_version_suffix(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter_map(|f| {
            let parts: Vec<_> = f.name().split('.').collect();
            let (_, before_action) = parts.split_last()?;
            let version = before_action.iter().find(|part| is_version_word(part))?;
            Some(function_error(
                f,
                "api-call-version-suffix",
                format!(
                    "`{version}` is a part of its own: make it a suffix of the action, \
                     such as `insert-{version}`"
                ),
            ))
        })
        .collect()
}

/// Actions the standard names a standard action for instead, with that
/// action and the rule.
const NOT_STANDARD_ACTIONS: &[(&str, &str, &str)] = &[
    ("create", "insert", "api-insert-not-create"),
    ("add", "insert", "api-insert-not-create"),
    ("generate", "insert", "api-insert-not-create"),
    ("new", "insert", "api-insert-not-create"),
    ("delete", "remove", "api-remove-not-delete"),
    ("destroy", "remove", "api-remove-not-delete"),
    ("release", "remove", "api-remove-not-delete"),
    ("erase", "remove", "api-action-standard"),
    ("fetch", "get", "api-action-standard"),
    ("read", "get", "api-action-standard"),
    ("retrieve", "get", "api-action-standard"),
    ("find", "search", "api-action-standard"),
    ("query", "search", "api-action-standard"),
    ("all", "list", "api-action-standard"),
    (
        "upsert",
        "insert, with `if: {exists: update}`",
        "api-action-standard",
    ),
    ("save", "insert or update", "api-action-standard"),
    ("put", "insert or update", "api-action-standard"),
    ("set", "update", "api-action-standard"),
    ("patch", "update", "api-action-standard"),
    ("edit", "update", "api-action-standard"),
    ("modify", "update", "api-action-standard"),
    ("change", "update", "api-action-standard"),
];

/// A call doesn't use an action that a standard one replaces, such as
/// `create` for `insert` or `delete` for `remove`:
/// `[api-insert-not-create]`, `[api-remove-not-delete]` and
/// `[api-action-standard]`.
pub fn call_actions_standard(schema: &Schema) -> Vec<ValidationError> {
    let mut errors = vec![];
    for f in schema.functions() {
        let action = base_action(f.name());
        let (word, many) = match action.strip_suffix("-many") {
            Some(word) => (word, "-many"),
            None => (action, ""),
        };
        if let Some((_, instead, rule)) = NOT_STANDARD_ACTIONS.iter().find(|(w, ..)| *w == word) {
            errors.push(function_error(
                f,
                rule,
                format!("`{action}` isn't a standard action: use {instead}{many}"),
            ));
        }
    }
    errors
}

const STANDARD_ACTIONS: &[&str] = &[
    "insert", "update", "remove", "get", "list", "search", "count", "ingest", "lookup",
];

/// A call says what it acts on, or searches by, in its resources and its
/// request, not in its action: `products.search` with a filter, not
/// `products.search-by-name`, and `users.password.update`, not
/// `users.update-password`: `[api-action-not-parameters]`.
pub fn call_actions_not_parameters(schema: &Schema) -> Vec<ValidationError> {
    let mut errors = vec![];
    for f in schema.functions() {
        let action = base_action(f.name());
        let qualified = STANDARD_ACTIONS.iter().any(|standard| {
            action
                .strip_prefix(standard)
                .and_then(|rest| rest.strip_prefix('-'))
                .is_some_and(|rest| rest != "many")
        });
        if qualified || action.contains("-by-") {
            errors.push(function_error(
                f,
                "api-action-not-parameters",
                format!(
                    "`{action}` names what the call acts on or searches by: \
                     put it in the resources or in the request"
                ),
            ));
        }
    }
    errors
}

// Bodies

/// Whether a type is written as an object on the wire.
enum Shape {
    Object,
    NotObject,
    /// A type parameter, or a type the schema doesn't hold.
    Unknown,
}

const OBJECT_PRIMITIVES: &[&str] = &[
    "std::collections::HashMap",
    "std::collections::BTreeMap",
    "indexmap::IndexMap",
    "serde_json::Map",
    "serde_json::Value",
];

/// `inner`, with a parameter of `outer`'s type replaced by its argument.
fn substitute(
    inner: &TypeReference,
    parameters: &[crate::TypeParameter],
    outer: &TypeReference,
) -> TypeReference {
    parameters
        .iter()
        .position(|parameter| parameter.name == inner.name)
        .and_then(|at| outer.arguments.get(at).cloned())
        .unwrap_or_else(|| inner.clone())
}

fn shape(typespace: &Typespace, type_ref: &TypeReference, depth: usize) -> Shape {
    if depth > 8 {
        return Shape::Unknown;
    }
    match typespace.get_type(&type_ref.name) {
        None => Shape::Unknown,
        Some(Type::Struct(s)) => {
            let tuple = matches!(s.fields, Fields::Unnamed(_));
            let fields: Vec<_> = s.fields().collect();
            // A newtype, or a transparent struct, is written as its field.
            if (s.transparent || tuple && fields.len() == 1) && !fields.is_empty() {
                let inner = substitute(&fields[0].type_ref, &s.parameters, type_ref);
                shape(typespace, &inner, depth + 1)
            } else if tuple && fields.len() > 1 {
                Shape::NotObject
            } else {
                Shape::Object
            }
        }
        Some(Type::Enum(e)) => {
            if matches!(
                e.name.as_str(),
                "std::option::Option" | "reflectapi::Option"
            ) {
                return Shape::NotObject;
            }
            let tagged_inside = matches!(
                e.representation,
                Representation::Internal { .. } | Representation::Adjacent { .. }
            );
            let external = matches!(e.representation, Representation::External);
            // A case without data is a string externally tagged, and `null`
            // untagged; an untagged case with one value is that value.
            let mut result = Shape::Object;
            for v in &e.variants {
                let untagged = v.untagged || !tagged_inside && !external;
                let case = match &v.fields {
                    Fields::None if !tagged_inside => Shape::NotObject,
                    Fields::Unnamed(fields) if untagged && fields.len() == 1 => {
                        let inner = substitute(&fields[0].type_ref, &e.parameters, type_ref);
                        shape(typespace, &inner, depth + 1)
                    }
                    Fields::Unnamed(fields) if untagged && fields.len() > 1 => Shape::NotObject,
                    _ => Shape::Object,
                };
                match case {
                    Shape::NotObject => return Shape::NotObject,
                    Shape::Unknown => result = Shape::Unknown,
                    Shape::Object => {}
                }
            }
            result
        }
        Some(Type::Primitive(p)) => {
            if OBJECT_PRIMITIVES.contains(&p.name.as_str()) {
                Shape::Object
            } else if let Some(fallback) = &p.fallback {
                let inner = substitute(fallback, &p.parameters, type_ref);
                shape(typespace, &inner, depth + 1)
            } else {
                Shape::NotObject
            }
        }
    }
}

/// A call's request is one object: `[api-arguments-in-body-object]`.
pub fn request_is_object(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter_map(|f| {
            // `()`: the call takes no arguments, and its body is left out.
            let input = f
                .input_type
                .as_ref()
                .filter(|input| input.name != "std::tuple::Tuple0")?;
            matches!(shape(schema.input_types(), input, 0), Shape::NotObject).then(|| {
                function_error(
                    f,
                    "api-arguments-in-body-object",
                    format!(
                        "the request, `{}`, isn't an object: wrap it in a struct",
                        input.name
                    ),
                )
            })
        })
        .collect()
}

/// A response's body is an object, the call's output and a value of its
/// error type alike: `[api-response-is-object]`.
pub fn response_is_object(schema: &Schema) -> Vec<ValidationError> {
    let mut errors = vec![];
    for f in schema.functions() {
        if let OutputType::Complete {
            output_type: Some(output),
        } = &f.output_type
        {
            if matches!(shape(schema.output_types(), output, 0), Shape::NotObject) {
                errors.push(function_error(
                    f,
                    "api-response-is-object",
                    format!(
                        "the response, `{}`, isn't an object: wrap it in a struct, \
                         a list under `items`",
                        output.name
                    ),
                ));
            }
        }
        if let Some(error_type) = &f.error_type {
            if matches!(
                shape(schema.output_types(), error_type, 0),
                Shape::NotObject
            ) {
                errors.push(function_error(
                    f,
                    "api-response-is-object",
                    format!(
                        "the error, `{}`, isn't an object: a case of it is written as a \
                         string; write its case in `kind`, `#[serde(tag = \"kind\")]`",
                        error_type.name
                    ),
                ));
            }
        }
    }
    errors
}

// Names in bodies and headers

/// A type reflectapi or the standard library defines, not the application.
fn is_library_type(name: &str) -> bool {
    name.starts_with("std::") || name.starts_with("reflectapi::")
}

/// The types that some call reads or writes as headers.
fn header_types(schema: &Schema) -> HashSet<&str> {
    schema
        .functions()
        .flat_map(|f| [f.input_headers.as_ref(), f.response_headers.as_ref()])
        .flatten()
        .map(|type_ref| type_ref.name.as_str())
        .collect()
}

/// Every type of the schema once, though it is in both typespaces.
fn types_once(schema: &Schema) -> impl Iterator<Item = &Type> {
    let mut seen = HashSet::new();
    schema
        .input_types()
        .types()
        .chain(schema.output_types().types())
        .filter(move |ty| seen.insert(ty.name()))
}

fn is_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Every field name and enum value of a request or a response is in
/// `snake_case`: `[api-name-snake-case]`. An enum value without data written in
/// capitals and digits only, such as `NZ`, passes, as a code from an outside
/// standard: `[api-enum-outside-codes-keep-form]`.
pub fn names_snake_case(schema: &Schema) -> Vec<ValidationError> {
    const RULE: &str = "api-name-snake-case";
    let headers = header_types(schema);
    let mut errors = vec![];
    let field = |type_name: &str, variant: Option<&str>, name: &str| {
        let pointer = match variant {
            Some(variant) => ValidationPointer::VariantField {
                type_name: type_name.into(),
                variant_name: variant.into(),
                name: name.into(),
            },
            None => ValidationPointer::Field {
                type_name: type_name.into(),
                name: name.into(),
            },
        };
        error(
            pointer,
            RULE,
            format!("the field name `{name}` isn't in snake_case"),
        )
    };
    for ty in types_once(schema) {
        let name = ty.name();
        if is_library_type(name) || headers.contains(name) {
            continue;
        }
        match ty {
            Type::Struct(s) => {
                if let Fields::Named(fields) = &s.fields {
                    for f in fields.iter().filter(|f| !f.flattened) {
                        if !is_snake_case(f.serde_name()) {
                            errors.push(field(name, None, f.serde_name()));
                        }
                    }
                }
            }
            Type::Enum(e) => {
                let tagged = !matches!(e.representation, Representation::None);
                let tags: &[&String] = match &e.representation {
                    Representation::Internal { tag } => &[tag],
                    Representation::Adjacent { tag, content } => &[tag, content],
                    _ => &[],
                };
                for tag in tags {
                    if !is_snake_case(tag) {
                        errors.push(error(
                            ValidationPointer::Type(name.into()),
                            RULE,
                            format!("the tag field `{tag}` isn't in snake_case"),
                        ));
                    }
                }
                for v in &e.variants {
                    let code = matches!(v.fields, Fields::None)
                        && v.serde_name()
                            .bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
                    if tagged && !v.untagged && !code && !is_snake_case(v.serde_name()) {
                        errors.push(error(
                            ValidationPointer::Variant {
                                type_name: name.into(),
                                name: v.name().into(),
                            },
                            RULE,
                            format!(
                                "the enum value `{}` isn't in snake_case: add \
                                 `#[serde(rename_all = \"snake_case\")]` to the enum",
                                v.serde_name()
                            ),
                        ));
                    }
                    if let Fields::Named(fields) = &v.fields {
                        for f in fields.iter().filter(|f| !f.flattened) {
                            if !is_snake_case(f.serde_name()) {
                                errors.push(field(name, Some(v.name()), f.serde_name()));
                            }
                        }
                    }
                }
            }
            Type::Primitive(_) => {}
        }
    }
    errors
}

/// An enum some of whose cases carry fields, and a call's error type, write
/// the case in a field named `kind` (`[api-enum-kind-tag]`), or `action`, as
/// an ingest item does; an enum each of whose cases holds one value, or none,
/// is written in serde's default form (`[api-enum-one-value-cases]`). An
/// untagged enum, whose forms its fields tell apart, isn't checked.
pub fn enums_kind_tag(schema: &Schema) -> Vec<ValidationError> {
    const RULE: &str = "api-enum-kind-tag";
    let error_types: HashSet<&str> = schema
        .functions()
        .filter_map(|f| f.error_type.as_ref())
        .map(|type_ref| type_ref.name.as_str())
        .collect();
    let headers = header_types(schema);
    let mut errors = vec![];
    for ty in types_once(schema) {
        let Type::Enum(e) = ty else { continue };
        if is_library_type(&e.name) || headers.contains(e.name.as_str()) {
            continue;
        }
        let tag = match &e.representation {
            Representation::Internal { tag } | Representation::Adjacent { tag, .. } => tag,
            Representation::External => "",
            Representation::None => continue,
        };
        let cases: Vec<_> = e.variants.iter().filter(|v| !v.untagged).collect();
        let without_data = cases.iter().all(|v| matches!(v.fields, Fields::None));
        let one_value = !cases.is_empty()
            && cases
                .iter()
                .all(|v| matches!(&v.fields, Fields::Unnamed(fields) if fields.len() == 1));
        let is_error = error_types.contains(e.name.as_str());
        let pointer = || ValidationPointer::Type(e.name.clone());
        if tag.is_empty() {
            if is_error {
                errors.push(error(
                    pointer(),
                    RULE,
                    "a call's error type: write its case in `kind`, \
                     `#[serde(tag = \"kind\")]`"
                        .into(),
                ));
            } else if !without_data && !one_value {
                errors.push(error(
                    pointer(),
                    RULE,
                    "some of its cases carry fields: write the case in `kind`, \
                     `#[serde(tag = \"kind\")]`"
                        .into(),
                ));
            }
        } else if tag != "kind" && tag != "action" {
            errors.push(error(
                pointer(),
                RULE,
                format!("its case is written in `{tag}`: name that field `kind`"),
            ));
        }
    }
    errors
}

/// The headers some call reads or writes, with the type that declares each.
fn header_fields(schema: &Schema) -> Vec<(&str, &str)> {
    let mut seen = HashSet::new();
    let mut fields = vec![];
    for f in schema.functions() {
        let declared = [
            (schema.input_types(), f.input_headers.as_ref()),
            (schema.output_types(), f.response_headers.as_ref()),
        ];
        for (typespace, type_ref) in declared {
            let Some(type_ref) = type_ref else { continue };
            if !seen.insert(type_ref.name.as_str()) {
                continue;
            }
            if let Some(Type::Struct(s)) = typespace.get_type(&type_ref.name) {
                for field in s.fields() {
                    fields.push((s.name.as_str(), field.serde_name()));
                }
            }
        }
    }
    fields
}

fn header_error(type_name: &str, header: &str, rule: &str, message: String) -> ValidationError {
    error(
        ValidationPointer::Field {
            type_name: type_name.into(),
            name: header.into(),
        },
        rule,
        message,
    )
}

/// A header's name is in lower-case `kebab-case`: `[api-header-name-case]`.
pub fn header_names_kebab_case(schema: &Schema) -> Vec<ValidationError> {
    header_fields(schema)
        .into_iter()
        .filter(|(_, header)| !is_kebab_case(header))
        .map(|(type_name, header)| {
            header_error(
                type_name,
                header,
                "api-header-name-case",
                format!("the header name `{header}` isn't in lower-case kebab-case"),
            )
        })
        .collect()
}

/// Headers starting with `x-` that proxies set, which a server may read.
const NETWORK_X_HEADERS: &[&str] = &[
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-real-ip",
];

/// An application's own header doesn't start with `x-`:
/// `[api-header-own-prefix]`.
pub fn header_names_no_x_prefix(schema: &Schema) -> Vec<ValidationError> {
    header_fields(schema)
        .into_iter()
        .filter(|(_, header)| {
            header.to_ascii_lowercase().starts_with("x-")
                && !NETWORK_X_HEADERS.contains(&header.to_ascii_lowercase().as_str())
        })
        .map(|(type_name, header)| {
            header_error(
                type_name,
                header,
                "api-header-own-prefix",
                format!(
                    "the header `{header}` starts with `x-`: start it with your \
                     organisation's prefix"
                ),
            )
        })
        .collect()
}

// Type names

fn last_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// A call name in `PascalCase`: `products.search-v2` is `ProductsSearchV2`.
fn pascal_case(call: &str) -> String {
    call.split(['.', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// The types of a typespace that something other than one call uses: a
/// call's request, response or error that another call uses too, or that a
/// type holds.
fn shared_types<'a>(typespace: &'a Typespace, uses: &[&'a TypeReference]) -> HashSet<&'a str> {
    fn arguments<'a>(type_ref: &'a TypeReference, into: &mut HashSet<&'a str>) {
        for argument in &type_ref.arguments {
            into.insert(argument.name.as_str());
            arguments(argument, into);
        }
    }
    let mut shared = HashSet::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for type_ref in uses {
        *counts.entry(type_ref.name.as_str()).or_default() += 1;
        arguments(type_ref, &mut shared);
    }
    shared.extend(
        counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name),
    );
    for ty in typespace.types() {
        let fields: Vec<&crate::Field> = match ty {
            Type::Struct(s) => s.fields().collect(),
            Type::Enum(e) => e.variants.iter().flat_map(|v| v.fields()).collect(),
            Type::Primitive(p) => {
                if let Some(fallback) = &p.fallback {
                    shared.insert(fallback.name.as_str());
                    arguments(fallback, &mut shared);
                }
                vec![]
            }
        };
        for field in fields {
            shared.insert(field.type_ref.name.as_str());
            arguments(&field.type_ref, &mut shared);
        }
    }
    shared
}

/// The words of a `PascalCase` name, in lower case: `O2TRegexRules` is
/// `o2t`, `regex`, `rules`.
fn pascal_words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words = vec![];
    let mut word = String::new();
    for (at, &c) in chars.iter().enumerate() {
        let next_lower = chars.get(at + 1).is_some_and(|n| n.is_ascii_lowercase());
        let previous = at.checked_sub(1).map(|p| chars[p]);
        let starts = c.is_ascii_uppercase()
            && previous.is_some_and(|p| {
                p.is_ascii_lowercase() || p.is_ascii_digit() || p.is_ascii_uppercase() && next_lower
            });
        if starts && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        if c.is_ascii_alphanumeric() {
            word.push(c.to_ascii_lowercase());
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// Whether a word is a version, `v2`, or `experimental`: words a call's
/// types may leave out.
fn is_version_word(word: &str) -> bool {
    word == "experimental"
        || word
            .strip_prefix('v')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// A word, singular, to compare `Offer` with `offers`.
fn singular(word: &str) -> &str {
    word.strip_suffix('s')
        .filter(|w| !w.is_empty())
        .unwrap_or(word)
}

/// Whether `words`, from `at`, start with `word`, maybe written as several
/// of them (`sign`, `in` for `signin`); returns where they end.
fn match_word(words: &[String], at: usize, word: &str) -> Option<usize> {
    let mut joined = String::new();
    for (end, w) in words.iter().enumerate().skip(at) {
        joined.push_str(w);
        if singular(&joined) == singular(word) {
            return Some(end + 1);
        }
        if joined.len() >= word.len() {
            return None;
        }
    }
    None
}

/// Whether a type's name, without its suffix, names `call`: it ends with the
/// call's action, and its words are words of the call, in their order.
/// `SetOverride` names `feature-flags.set-override`, `OfferRepairerSearch`
/// names `offers.repairer.search`, and `UsersSignIn` names `users.signin`.
fn names_call(prefix: &str, call: &str) -> bool {
    let split = |text: &str| -> Vec<String> {
        text.split(['.', '-', '_'])
            .map(str::to_ascii_lowercase)
            .filter(|w| !w.is_empty() && !is_version_word(w))
            .collect()
    };
    let type_words: Vec<String> = pascal_words(prefix)
        .into_iter()
        .filter(|w| !is_version_word(w))
        .collect();
    let call_words = split(call);
    let action: String = split(base_action(call)).concat();
    if type_words.is_empty() || !type_words.concat().ends_with(&action) {
        return false;
    }
    // Each type word, or run of them, is a call word, or a run of call
    // words, in the call's order.
    let mut at = 0;
    let mut call_at = 0;
    while at < type_words.len() {
        let mut matched = None;
        for (c, _) in call_words.iter().enumerate().skip(call_at) {
            for end in c + 1..=call_words.len() {
                let run = call_words[c..end].concat();
                if let Some(next) = match_word(&type_words, at, &run) {
                    matched = Some((next, end));
                    break;
                }
            }
            if matched.is_some() {
                break;
            }
        }
        let Some((next, call_next)) = matched else {
            return false;
        };
        at = next;
        call_at = call_next;
    }
    true
}

/// A type that one call alone uses as its request, response or error, and
/// that is named as one, with the suffix `Request`, `Response` or `Error`,
/// names that call: `ProductsSearchRequest`, or `SearchRequest` in a module
/// of its own, for `products.search`, and not `ProductsListRequest`:
/// `[api-type-named-after-call]`.
///
/// The rule names every such type after its call; this checks only the types
/// named as a call's, so that one a call uses as it is, such as a `Product`,
/// passes.
pub fn call_type_names(schema: &Schema) -> Vec<ValidationError> {
    let inputs: Vec<&TypeReference> = schema
        .functions()
        .filter_map(|f| f.input_type.as_ref())
        .collect();
    let outputs: Vec<&TypeReference> = schema
        .functions()
        .flat_map(|f| {
            let output = match &f.output_type {
                OutputType::Complete { output_type } => output_type.as_ref(),
                OutputType::Stream { .. } => None,
            };
            [output, f.error_type.as_ref()]
        })
        .flatten()
        .collect();
    let shared_inputs = shared_types(schema.input_types(), &inputs);
    let shared_outputs = shared_types(schema.output_types(), &outputs);

    let mut errors = vec![];
    for f in schema.functions() {
        let output = match &f.output_type {
            OutputType::Complete { output_type } => output_type.as_ref(),
            OutputType::Stream { .. } => None,
        };
        let roles = [
            (f.input_type.as_ref(), "Request", "request", &shared_inputs),
            (output, "Response", "response", &shared_outputs),
            (f.error_type.as_ref(), "Error", "error", &shared_outputs),
        ];
        for (type_ref, suffix, role, shared) in roles {
            let Some(type_ref) = type_ref else { continue };
            let name = type_ref.name.as_str();
            if is_library_type(name) || !type_ref.arguments.is_empty() || shared.contains(name) {
                continue;
            }
            let short = last_segment(name);
            let Some(prefix) = short.strip_suffix(suffix).filter(|p| !p.is_empty()) else {
                continue;
            };
            if !names_call(prefix, f.name()) {
                let expected = format!("{}{suffix}", pascal_case(f.name()));
                errors.push(function_error(
                    f,
                    "api-type-named-after-call",
                    format!(
                        "the {role}, `{name}`, is named as a call's but not after \
                         this one: name it `{expected}`"
                    ),
                ));
            }
        }
    }
    errors
}

/// A type's name doesn't end with the kind of type it is, `Enum` or
/// `Struct`: `[api-type-no-kind-suffix]`.
pub fn type_names_no_kind_suffix(schema: &Schema) -> Vec<ValidationError> {
    types_once(schema)
        .filter(|ty| !is_library_type(ty.name()))
        .filter_map(|ty| {
            let short = last_segment(ty.name());
            let suffix = ["Enum", "Struct"]
                .into_iter()
                .find(|suffix| short.len() > suffix.len() && short.ends_with(suffix))?;
            Some(error(
                ValidationPointer::Type(ty.name().into()),
                "api-type-no-kind-suffix",
                format!("`{short}` ends with `{suffix}`, the kind of type it is"),
            ))
        })
        .collect()
}

/// A type's name has at most two modules before it, a version module, such
/// as `v2`, not counted: `[api-type-path-depth]`.
pub fn type_path_depth(schema: &Schema) -> Vec<ValidationError> {
    types_once(schema)
        .filter(|ty| !is_library_type(ty.name()))
        .filter_map(|ty| {
            let modules = ty
                .name()
                .split("::")
                .filter(|part| !is_version_word(part))
                .count()
                .saturating_sub(1);
            (modules > 2).then(|| {
                error(
                    ValidationPointer::Type(ty.name().into()),
                    "api-type-path-depth",
                    format!(
                        "`{}` has {modules} modules before its name: rename the \
                         modules with `glob_rename_types`",
                        ty.name()
                    ),
                )
            })
        })
        .collect()
}

/// Module names that say how the server's code is organised.
const IMPLEMENTATION_MODULES: &[&str] = &[
    "model",
    "models",
    "type",
    "types",
    "request",
    "requests",
    "response",
    "responses",
    "error",
    "errors",
    "dto",
    "dtos",
];

/// A module in a type's name doesn't say how the server's code is organised,
/// such as `models` or `errors`: `[api-type-modules-not-implementation]`.
pub fn type_modules_not_implementation(schema: &Schema) -> Vec<ValidationError> {
    types_once(schema)
        .filter(|ty| !is_library_type(ty.name()))
        .filter_map(|ty| {
            let parts: Vec<_> = ty.name().split("::").collect();
            let (_, modules) = parts.split_last()?;
            let module = modules
                .iter()
                .find(|module| IMPLEMENTATION_MODULES.contains(module))?;
            Some(error(
                ValidationPointer::Type(ty.name().into()),
                "api-type-modules-not-implementation",
                format!(
                    "the module `{module}` says how the code is organised: rename it \
                     away with `glob_rename_types`"
                ),
            ))
        })
        .collect()
}

/// A case of a call's error type doesn't end with `Error`:
/// `[api-error-case-no-error-suffix]`.
pub fn error_cases_no_error_suffix(schema: &Schema) -> Vec<ValidationError> {
    let mut seen = HashSet::new();
    let mut errors = vec![];
    for type_ref in schema.functions().filter_map(|f| f.error_type.as_ref()) {
        if is_library_type(&type_ref.name) || !seen.insert(type_ref.name.as_str()) {
            continue;
        }
        let Some(Type::Enum(e)) = schema.output_types().get_type(&type_ref.name) else {
            continue;
        };
        for v in &e.variants {
            // The derive may hold the name as `rename_all` writes it.
            let suffixed = |name: &str| {
                name.len() > "Error".len() && name.ends_with("Error")
                    || name.len() > "_error".len() && name.ends_with("_error")
            };
            if suffixed(v.name()) || suffixed(v.serde_name()) {
                errors.push(error(
                    ValidationPointer::Variant {
                        type_name: e.name.clone(),
                        name: v.name().into(),
                    },
                    "api-error-case-no-error-suffix",
                    format!(
                        "`{}` ends with `Error`, which its type says",
                        v.serde_name()
                    ),
                ));
            }
        }
    }
    errors
}
