use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LanguageSpecificTypeCodegenConfig {
    pub rust: RustTypeCodegenConfig,
    // Add other languages as required
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RustTypeCodegenConfig {
    pub additional_derives: BTreeSet<String>,
}
