use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LanguageSpecificTypeCodegenConfig {
    pub rust: RustTypeCodegenConfig,
    #[serde(skip_serializing, default)]
    pub python: PythonTypeCodegenConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RustTypeCodegenConfig {
    pub additional_derives: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PythonTypeCodegenConfig {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub type_hint: Option<String>,

    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub imports: BTreeSet<String>,

    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub runtime_imports: BTreeSet<String>,

    #[serde(default)]
    pub provided_by_runtime: bool,

    #[serde(default)]
    pub ignore_type_arguments: bool,
}

impl PythonTypeCodegenConfig {
    pub fn is_empty(&self) -> bool {
        self.type_hint.is_none()
            && self.imports.is_empty()
            && self.runtime_imports.is_empty()
            && !self.provided_by_runtime
            && !self.ignore_type_arguments
    }
}

impl LanguageSpecificTypeCodegenConfig {
    pub fn is_serialization_default(&self) -> bool {
        self.rust == RustTypeCodegenConfig::default()
    }
}
