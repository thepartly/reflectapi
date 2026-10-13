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
        }
    }
    for ty in schema.output_types.types() {
        let crate::Type::Enum(enm) = ty else { continue };
        if enm.open {
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
            anyhow::ensure!(
                matches!(enm.representation, crate::Representation::Internal { .. } | crate::Representation::Adjacent { .. })
                    && matches!(variant.fields, crate::Fields::None)
                    && !variant.untagged,
                "{}::{}: serde(other) requires a unit variant in an internally or adjacently tagged enum",
                enm.name, variant.name,
            );
        }
        anyhow::ensure!(
            other_count <= 1,
            "{}: enum has more than one serde(other) fallback",
            enm.name
        );
    }
    Ok(())
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
