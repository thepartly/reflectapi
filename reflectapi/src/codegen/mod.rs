mod format;
pub mod openapi;
pub mod python;
pub mod rust;
// Compiler-only helpers (symbol identity, schema-ID assignment,
// normalization, semantic IR). Used by Python codegen today, available
// to other backends; intentionally not re-exported as part of
// `reflectapi`'s public API so the IR can keep evolving.
pub(crate) mod schema;
pub mod typescript;

use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hasher},
    path::PathBuf,
};

use self::format::format_with;

/// Validate client enum policies, including schemas loaded directly from JSON.
pub(super) fn validate_response_enums(schema: &crate::Schema) -> anyhow::Result<()> {
    for ty in schema.input_types.types() {
        if let crate::Type::Enum(enm) = ty {
            anyhow::ensure!(
                !enm.open,
                "{}: open enums are only supported in response types",
                enm.name
            );
            anyhow::ensure!(
                enm.variants.iter().all(|variant| !variant.other),
                "{}: serde(other) fallback policies are only supported in response types",
                enm.name
            );
        }
    }
    for ty in schema.output_types.types() {
        let crate::Type::Enum(enm) = ty else { continue };
        if enm.open {
            anyhow::ensure!(
                enm.variants.iter().all(|variant| !variant.other),
                "{}: open response enums cannot contain serde(other); choose either a declared fallback or a rich unknown response",
                enm.name
            );
            anyhow::ensure!(
                enm.variants
                    .iter()
                    .all(|variant| variant.discriminant.is_none()),
                "{}: open response enums require string tags, not numeric discriminants",
                enm.name
            );
            anyhow::ensure!(
                !matches!(enm.representation, crate::Representation::None)
                    && enm.variants.iter().all(|variant| !variant.untagged),
                "{}: open response enums require a tagged representation without untagged variants",
                enm.name,
            );
        }
        let mut other_count = 0;
        for variant in enm.variants.iter().filter(|variant| variant.other) {
            other_count += 1;
            // Serde also honours `other` in an externally tagged enum, where an
            // unknown string selects it. Only an all-unit enum is accepted there:
            // it is a plain string on the wire, so every client can decode it.
            let supported = match enm.representation {
                crate::Representation::Internal { .. } | crate::Representation::Adjacent { .. } => {
                    true
                }
                crate::Representation::External => enm.variants.iter().all(|variant| {
                    matches!(variant.fields, crate::Fields::None) && variant.discriminant.is_none()
                }),
                crate::Representation::None => false,
            };
            anyhow::ensure!(
                supported && matches!(variant.fields, crate::Fields::None) && !variant.untagged,
                "{}::{}: serde(other) requires a unit variant in an internally or adjacently tagged enum, or in an externally tagged enum whose variants are all units",
                enm.name, variant.name,
            );
        }
        anyhow::ensure!(
            other_count <= 1,
            "{}: enum has more than one serde(other) fallback",
            enm.name
        );
    }
    for ty in schema.output_types.types() {
        let fields: Box<dyn Iterator<Item = &crate::Field> + '_> = match ty {
            crate::Type::Struct(strukt) => Box::new(strukt.fields.iter()),
            crate::Type::Enum(enm) => Box::new(
                enm.variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter()),
            ),
            crate::Type::Primitive(_) => continue,
        };
        for field in fields.filter(|field| field.flattened && !field.hidden) {
            if let Some(name) =
                flattened_external_open_enum(schema, &field.type_ref, &mut HashSet::new())
            {
                anyhow::bail!(
                    "{}::{}: flattening open externally tagged response enum {name} is unsupported; use an internally or adjacently tagged enum",
                    ty.name(), field.name
                );
            }
        }
    }
    for ty in schema.output_types.types() {
        match ty {
            crate::Type::Struct(strukt) => {
                let fragments = flattened_enum_fragments_for_fields(
                    schema,
                    strukt.fields.iter(),
                    &mut HashSet::new(),
                );
                anyhow::ensure!(
                    !fragments.has_policy || fragments.count < 2,
                    "{}: a flattened response-policy enum cannot share an object with another flattened enum; use named fields instead of flattening them",
                    strukt.name
                );
            }
            crate::Type::Enum(enm) => {
                for variant in &enm.variants {
                    let fragments = flattened_enum_fragments_for_fields(
                        schema,
                        variant.fields.iter(),
                        &mut HashSet::new(),
                    );
                    anyhow::ensure!(
                        !fragments.has_policy || fragments.count < 2,
                        "{}::{}: a flattened response-policy enum cannot share an object with another flattened enum; use named fields instead of flattening them",
                        enm.name,
                        variant.name
                    );
                }
            }
            crate::Type::Primitive(_) => {}
        }
    }
    Ok(())
}

fn bind_type_parameters(
    child: &crate::TypeReference,
    parent: &crate::Type,
    parent_reference: &crate::TypeReference,
) -> crate::TypeReference {
    if let Some(index) = parent
        .parameters()
        .position(|parameter| parameter.name == child.name)
    {
        if let Some(argument) = parent_reference.arguments.get(index) {
            return argument.clone();
        }
    }
    crate::TypeReference::new(
        child.name.clone(),
        child
            .arguments
            .iter()
            .map(|argument| bind_type_parameters(argument, parent, parent_reference))
            .collect(),
    )
}

/// Follow only fields whose wire value is flattened into the containing map.
fn flattened_external_open_enum(
    schema: &crate::Schema,
    reference: &crate::TypeReference,
    seen: &mut HashSet<String>,
) -> Option<String> {
    if matches!(
        reference.name.as_str(),
        "std::option::Option" | "reflectapi::Option" | "std::boxed::Box"
    ) {
        return reference
            .arguments
            .first()
            .and_then(|argument| flattened_external_open_enum(schema, argument, seen));
    }
    let ty = schema.output_types.get_type(&reference.name)?;
    if let crate::Type::Enum(enm) = ty {
        if enm.open && matches!(enm.representation, crate::Representation::External) {
            return Some(enm.name.clone());
        }
    }
    if !seen.insert(reference.name.clone()) {
        return None;
    }
    let fields: Box<dyn Iterator<Item = &crate::Field> + '_> = match ty {
        crate::Type::Struct(strukt) if strukt.is_alias() => Box::new(strukt.fields.iter()),
        crate::Type::Struct(strukt) => {
            Box::new(strukt.fields.iter().filter(|field| field.flattened))
        }
        crate::Type::Enum(enm) => Box::new(
            enm.variants
                .iter()
                .flat_map(|variant| variant.fields.iter().filter(|field| field.flattened)),
        ),
        crate::Type::Primitive(_) => Box::new(std::iter::empty()),
    };
    let found = fields.filter(|field| !field.hidden).find_map(|field| {
        let child = bind_type_parameters(&field.type_ref, ty, reference);
        flattened_external_open_enum(schema, &child, seen)
    });
    seen.remove(&reference.name);
    found
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct FlattenedEnumFragments {
    count: usize,
    has_policy: bool,
}

impl FlattenedEnumFragments {
    fn merge(self, other: Self) -> Self {
        Self {
            count: self.count + other.count,
            has_policy: self.has_policy || other.has_policy,
        }
    }
}

fn flattened_enum_fragments_for_fields<'a>(
    schema: &crate::Schema,
    fields: impl Iterator<Item = &'a crate::Field>,
    seen: &mut HashSet<String>,
) -> FlattenedEnumFragments {
    fields
        .filter(|field| field.flattened && !field.hidden)
        .map(|field| flattened_enum_fragments(schema, &field.type_ref, seen))
        .fold(
            FlattenedEnumFragments::default(),
            FlattenedEnumFragments::merge,
        )
}

fn flattened_enum_fragments(
    schema: &crate::Schema,
    reference: &crate::TypeReference,
    seen: &mut HashSet<String>,
) -> FlattenedEnumFragments {
    if matches!(
        reference.name.as_str(),
        "std::option::Option" | "reflectapi::Option" | "std::boxed::Box"
    ) {
        return reference
            .arguments
            .first()
            .map_or(FlattenedEnumFragments::default(), |argument| {
                flattened_enum_fragments(schema, argument, seen)
            });
    }
    let Some(ty) = schema.output_types.get_type(&reference.name) else {
        return FlattenedEnumFragments::default();
    };
    if !seen.insert(reference.name.clone()) {
        return FlattenedEnumFragments::default();
    }
    let fragments = match ty {
        crate::Type::Struct(strukt) if strukt.is_alias() => strukt
            .fields
            .iter()
            .filter(|field| !field.hidden)
            .map(|field| {
                let child = bind_type_parameters(&field.type_ref, ty, reference);
                flattened_enum_fragments(schema, &child, seen)
            })
            .fold(
                FlattenedEnumFragments::default(),
                FlattenedEnumFragments::merge,
            ),
        crate::Type::Struct(strukt) => strukt
            .fields
            .iter()
            .filter(|field| field.flattened && !field.hidden)
            .map(|field| {
                let child = bind_type_parameters(&field.type_ref, ty, reference);
                flattened_enum_fragments(schema, &child, seen)
            })
            .fold(
                FlattenedEnumFragments::default(),
                FlattenedEnumFragments::merge,
            ),
        crate::Type::Enum(enm) => {
            let own = FlattenedEnumFragments {
                count: 1,
                has_policy: enm.open || enm.variants.iter().any(|variant| variant.other),
            };
            let nested = enm
                .variants
                .iter()
                .map(|variant| {
                    variant
                        .fields
                        .iter()
                        .filter(|field| field.flattened && !field.hidden)
                        .map(|field| {
                            let child = bind_type_parameters(&field.type_ref, ty, reference);
                            flattened_enum_fragments(schema, &child, seen)
                        })
                        .fold(
                            FlattenedEnumFragments::default(),
                            FlattenedEnumFragments::merge,
                        )
                })
                .max_by_key(|fragment| (fragment.count, fragment.has_policy))
                .unwrap_or_default();
            own.merge(nested)
        }
        crate::Type::Primitive(_) => FlattenedEnumFragments::default(),
    };
    seen.remove(&reference.name);
    fragments
}

#[derive(Debug, Default)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    pub format: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    pub typecheck: bool,
    /// Include tracing in the generated code.
    pub tracing: bool,
    pub shared_modules: Vec<String>,
}

fn tmp_path(src: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    hasher.write(src.as_bytes());
    let hash = hasher.finish();

    std::env::temp_dir().join(format!("reflectapi-{hash}"))
}

// Comes in pairs, anything between them is boilerplate and can be removed for tests snapshots.
const START_BOILERPLATE: &str = "/* <----- */";
const END_BOILERPLATE: &str = "/* -----> */";

#[doc(hidden)]
pub fn strip_boilerplate(src: &str) -> String {
    let mut stripped = String::new();
    let mut skip = false;
    for line in src.lines() {
        if line.contains(START_BOILERPLATE) {
            assert!(!skip, "nested start boilerplate markers");
            skip = true;
            continue;
        }

        if line.contains(END_BOILERPLATE) {
            assert!(skip, "unmatched end boilerplate marker");
            skip = false;
            continue;
        }

        if !skip {
            stripped.push_str(line);
            stripped.push('\n');
        }
    }

    if skip {
        panic!("unmatched boilerplate marker");
    }

    stripped
}

#[cfg(test)]
mod enum_policy_tests {
    fn two_policy_enums_schema() -> crate::Schema {
        let mut schema = crate::Schema::default();
        for (name, tag) in [("OpenA", "kind_a"), ("OpenB", "kind_b")] {
            let mut enm = crate::Enum::new(name.into());
            enm.open = true;
            enm.representation = crate::Representation::Internal { tag: tag.into() };
            enm.variants.push(crate::Variant::new("Ready".into()));
            schema.output_types.insert_type(enm.into());
        }
        schema
    }

    fn external_open_schema() -> crate::Schema {
        let mut enm = crate::Enum::new("OpenExternal".into());
        enm.open = true;
        enm.variants.push(crate::Variant::new("Ready".into()));
        let mut schema = crate::Schema::default();
        schema.output_types.insert_type(enm.into());
        schema
    }

    fn flattened_field(name: &str, type_ref: crate::TypeReference) -> crate::Field {
        let mut field = crate::Field::new(name.into(), type_ref);
        field.flattened = true;
        field
    }

    fn assert_all_sdk_reject_flattened_external(schema: crate::Schema) {
        let schema: crate::Schema =
            serde_json::from_value(serde_json::to_value(schema).unwrap()).unwrap();
        for error in [
            super::rust::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::typescript::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::python::generate(schema, &Default::default()).unwrap_err(),
        ] {
            let message = error.to_string();
            assert!(
                message.contains("flattening open externally tagged response enum OpenExternal"),
                "{message}"
            );
            assert!(
                message.contains("internally or adjacently tagged"),
                "{message}"
            );
        }
    }

    #[test]
    fn all_sdk_generators_reject_flattened_external_open_from_json() {
        for optional in [false, true] {
            let mut schema = external_open_schema();
            let type_ref = crate::TypeReference::new("OpenExternal", vec![]);
            let type_ref = if optional {
                crate::TypeReference::new("std::option::Option", vec![type_ref])
            } else {
                type_ref
            };
            let mut envelope = crate::Struct::new("Envelope");
            envelope.fields = crate::Fields::Named(vec![flattened_field("value", type_ref)]);
            schema.output_types.insert_type(envelope.into());
            assert_all_sdk_reject_flattened_external(schema);
        }

        let mut schema = external_open_schema();
        let mut wrapper = crate::Struct::new("ExternalAlias");
        wrapper.transparent = true;
        wrapper.fields = crate::Fields::Named(vec![crate::Field::new(
            "inner".into(),
            crate::TypeReference::new("OpenExternal", vec![]),
        )]);
        schema.output_types.insert_type(wrapper.into());
        let mut envelope = crate::Struct::new("Envelope");
        envelope.fields = crate::Fields::Named(vec![flattened_field(
            "value",
            crate::TypeReference::new("ExternalAlias", vec![]),
        )]);
        schema.output_types.insert_type(envelope.into());
        assert_all_sdk_reject_flattened_external(schema);

        let mut schema = external_open_schema();
        let mut wrapper = crate::Struct::new("GenericAlias");
        wrapper.transparent = true;
        wrapper.parameters.push(crate::TypeParameter {
            name: "T".into(),
            description: String::new(),
        });
        wrapper.fields = crate::Fields::Named(vec![crate::Field::new(
            "inner".into(),
            crate::TypeReference::new("T", vec![]),
        )]);
        schema.output_types.insert_type(wrapper.into());
        let mut envelope = crate::Struct::new("Envelope");
        envelope.fields = crate::Fields::Named(vec![flattened_field(
            "value",
            crate::TypeReference::new(
                "GenericAlias",
                vec![crate::TypeReference::new("OpenExternal", vec![])],
            ),
        )]);
        schema.output_types.insert_type(envelope.into());
        assert_all_sdk_reject_flattened_external(schema);

        let mut schema = external_open_schema();
        let mut variant = crate::Variant::new("Wrapped".into());
        variant.fields = crate::Fields::Named(vec![flattened_field(
            "value",
            crate::TypeReference::new("OpenExternal", vec![]),
        )]);
        let mut outer = crate::Enum::new("Outer".into());
        outer.variants.push(variant);
        schema.output_types.insert_type(outer.into());
        assert_all_sdk_reject_flattened_external(schema);
    }

    #[test]
    fn flattened_struct_with_ordinary_open_external_child_is_allowed() {
        let mut schema = external_open_schema();
        let mut nested = crate::Struct::new("Nested");
        nested.fields = crate::Fields::Named(vec![crate::Field::new(
            "value".into(),
            crate::TypeReference::new("OpenExternal", vec![]),
        )]);
        schema.output_types.insert_type(nested.into());
        let mut envelope = crate::Struct::new("Envelope");
        envelope.fields = crate::Fields::Named(vec![flattened_field(
            "nested",
            crate::TypeReference::new("Nested", vec![]),
        )]);
        schema.output_types.insert_type(envelope.into());
        let schema: crate::Schema =
            serde_json::from_value(serde_json::to_value(schema).unwrap()).unwrap();
        super::validate_response_enums(&schema).unwrap();
    }

    #[test]
    fn all_sdk_generators_reject_two_flattened_response_policies_from_json() {
        let mut schema = two_policy_enums_schema();
        let mut parent = crate::Struct::new("Parent");
        parent.fields = crate::Fields::Named(vec![
            flattened_field("first", crate::TypeReference::new("OpenA", vec![])),
            flattened_field("second", crate::TypeReference::new("OpenB", vec![])),
        ]);
        schema.output_types.insert_type(parent.into());
        let schema: crate::Schema =
            serde_json::from_value(serde_json::to_value(schema).unwrap()).unwrap();
        for error in [
            super::rust::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::typescript::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::python::generate(schema, &Default::default()).unwrap_err(),
        ] {
            let message = error.to_string();
            assert!(
                message.contains("a flattened response-policy enum cannot share an object"),
                "{message}"
            );
            assert!(message.contains("use named fields"), "{message}");
        }
    }

    #[test]
    fn multiple_flattened_policies_are_counted_through_structs_and_per_variant() {
        let mut schema = two_policy_enums_schema();
        for (name, enum_name) in [("PartA", "OpenA"), ("PartB", "OpenB")] {
            let mut part = crate::Struct::new(name);
            part.fields = crate::Fields::Named(vec![flattened_field(
                "value",
                crate::TypeReference::new(enum_name, vec![]),
            )]);
            schema.output_types.insert_type(part.into());
        }
        let mut parent = crate::Struct::new("Parent");
        parent.fields = crate::Fields::Named(vec![
            flattened_field("first", crate::TypeReference::new("PartA", vec![])),
            flattened_field("second", crate::TypeReference::new("PartB", vec![])),
        ]);
        schema.output_types.insert_type(parent.into());
        assert!(super::validate_response_enums(&schema)
            .unwrap_err()
            .to_string()
            .contains("Parent: a flattened response-policy enum cannot share an object"));

        let mut schema = two_policy_enums_schema();
        let mut outer = crate::Enum::new("Outer".into());
        for (name, enum_name) in [("First", "OpenA"), ("Second", "OpenB")] {
            let mut variant = crate::Variant::new(name.into());
            variant.fields = crate::Fields::Named(vec![flattened_field(
                "value",
                crate::TypeReference::new(enum_name, vec![]),
            )]);
            outer.variants.push(variant);
        }
        schema.output_types.insert_type(outer.into());
        super::validate_response_enums(&schema).unwrap();

        let mut schema = two_policy_enums_schema();
        let crate::Type::Enum(second) = schema.output_types.get_type_mut("OpenB").unwrap() else {
            panic!("expected OpenB enum");
        };
        second.open = false;
        let mut other = crate::Variant::new("Other".into());
        other.other = true;
        second.variants.push(other);
        let mut parent = crate::Struct::new("Parent");
        parent.fields = crate::Fields::Named(vec![
            flattened_field("first", crate::TypeReference::new("OpenA", vec![])),
            flattened_field("second", crate::TypeReference::new("OpenB", vec![])),
        ]);
        schema.output_types.insert_type(parent.into());
        assert!(super::validate_response_enums(&schema)
            .unwrap_err()
            .to_string()
            .contains("a flattened response-policy enum cannot share an object"));
    }

    #[test]
    fn all_sdk_generators_reject_closed_and_open_flattened_enums_in_either_order() {
        for (first, second) in [("OpenA", "OpenB"), ("OpenB", "OpenA")] {
            let mut schema = two_policy_enums_schema();
            let crate::Type::Enum(closed) = schema.output_types.get_type_mut("OpenA").unwrap()
            else {
                panic!("expected OpenA enum");
            };
            closed.open = false;
            let mut parent = crate::Struct::new("Parent");
            parent.fields = crate::Fields::Named(vec![
                flattened_field("first", crate::TypeReference::new(first, vec![])),
                flattened_field("second", crate::TypeReference::new(second, vec![])),
            ]);
            schema.output_types.insert_type(parent.into());
            let schema: crate::Schema =
                serde_json::from_value(serde_json::to_value(schema).unwrap()).unwrap();
            for error in [
                super::rust::generate(schema.clone(), &Default::default()).unwrap_err(),
                super::typescript::generate(schema.clone(), &Default::default()).unwrap_err(),
                super::python::generate(schema, &Default::default()).unwrap_err(),
            ] {
                let message = error.to_string();
                assert!(
                    message.contains("a flattened response-policy enum cannot share an object"),
                    "{message}"
                );
            }
        }

        let mut schema = two_policy_enums_schema();
        for name in ["OpenA", "OpenB"] {
            let crate::Type::Enum(closed) = schema.output_types.get_type_mut(name).unwrap() else {
                panic!("expected enum");
            };
            closed.open = false;
        }
        let mut parent = crate::Struct::new("Parent");
        parent.fields = crate::Fields::Named(vec![
            flattened_field("first", crate::TypeReference::new("OpenA", vec![])),
            flattened_field("second", crate::TypeReference::new("OpenB", vec![])),
        ]);
        schema.output_types.insert_type(parent.into());
        super::validate_response_enums(&schema).unwrap();
    }

    #[test]
    fn all_sdk_generators_reject_request_fallback_policies_from_json() {
        let mut enm = crate::Enum::new("RequestFallback".into());
        enm.representation = crate::Representation::Internal { tag: "kind".into() };
        let mut variant = crate::Variant::new("Other".into());
        variant.other = true;
        enm.variants.push(variant);
        let mut schema = crate::Schema::default();
        schema.input_types.insert_type(enm.into());
        for error in [
            super::rust::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::typescript::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::python::generate(schema, &Default::default()).unwrap_err(),
        ] {
            assert!(error
                .to_string()
                .contains("only supported in response types"));
        }
    }

    #[test]
    fn all_sdk_generators_reject_mixed_open_and_other_from_json() {
        let mut enm = crate::Enum::new("MixedPolicy".into());
        enm.open = true;
        enm.representation = crate::Representation::Internal { tag: "kind".into() };
        let mut variant = crate::Variant::new("Other".into());
        variant.other = true;
        enm.variants.push(variant);
        let mut schema = crate::Schema::default();
        schema.output_types.insert_type(enm.into());
        for error in [
            super::rust::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::typescript::generate(schema.clone(), &Default::default()).unwrap_err(),
            super::python::generate(schema, &Default::default()).unwrap_err(),
        ] {
            assert!(error.to_string().contains("cannot contain serde(other)"));
        }
    }

    #[test]
    fn all_sdk_generators_reject_unsupported_policies_from_json() {
        let mut enm = crate::Enum::new("Unsupported".into());
        enm.open = true;
        enm.representation = crate::Representation::None;
        let mut schema = crate::Schema::default();
        schema.output_types.insert_type(enm.into());
        assert!(super::rust::generate(schema.clone(), &Default::default()).is_err());
        assert!(super::typescript::generate(schema.clone(), &Default::default()).is_err());
        assert!(super::python::generate(schema, &Default::default()).is_err());
    }
}
