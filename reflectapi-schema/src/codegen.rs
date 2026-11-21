use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct LanguageSpecificTypeCodegenConfig {
    pub rust: RustTypeCodegenConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct RustTypeCodegenConfig {
    pub additional_derives: BTreeSet<String>,
}

impl LanguageSpecificTypeCodegenConfig {
    pub fn is_serialization_default(&self) -> bool {
        self.rust == RustTypeCodegenConfig::default()
    }
}
