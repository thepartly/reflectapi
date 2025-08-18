/// Stable, unique identifier for symbols that persists across all pipeline stages
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SymbolId {
    pub kind: SymbolKind,
    pub path: Vec<String>,
    pub disambiguator: u32,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum SymbolKind {
    Struct,
    Enum,
    TypeAlias,
    Endpoint,
    Variant,
    Field,
    Primitive,
}

impl Default for SymbolId {
    fn default() -> Self {
        Self {
            kind: SymbolKind::Struct,
            path: vec!["unknown".to_string()],
            disambiguator: 0,
        }
    }
}

impl SymbolId {
    /// Create a new symbol ID with path and kind
    pub fn new(kind: SymbolKind, path: Vec<String>) -> Self {
        Self {
            kind,
            path,
            disambiguator: 0,
        }
    }

    /// Create a new symbol ID with disambiguation
    pub fn with_disambiguator(kind: SymbolKind, path: Vec<String>, disambiguator: u32) -> Self {
        Self {
            kind,
            path,
            disambiguator,
        }
    }

    /// Check if this is an unknown/default symbol ID
    pub fn is_unknown(&self) -> bool {
        self.path.len() == 1 && self.path[0] == "unknown"
    }

    /// Create a symbol ID for a struct
    pub fn struct_id(path: Vec<String>) -> Self {
        Self::new(SymbolKind::Struct, path)
    }

    /// Create a symbol ID for an enum
    pub fn enum_id(path: Vec<String>) -> Self {
        Self::new(SymbolKind::Enum, path)
    }

    /// Create a symbol ID for an endpoint/function
    pub fn endpoint_id(path: Vec<String>) -> Self {
        Self::new(SymbolKind::Endpoint, path)
    }

    /// Create a symbol ID for a variant
    pub fn variant_id(enum_path: Vec<String>, variant_name: String) -> Self {
        let mut path = enum_path;
        path.push(variant_name);
        Self::new(SymbolKind::Variant, path)
    }

    /// Create a symbol ID for a field
    pub fn field_id(parent_path: Vec<String>, field_name: String) -> Self {
        let mut path = parent_path;
        path.push(field_name);
        Self::new(SymbolKind::Field, path)
    }

    /// Get the simple name (last component of path)
    pub fn name(&self) -> Option<&str> {
        self.path.last().map(|s| s.as_str())
    }

    /// Get the qualified name as a dot-separated string
    pub fn qualified_name(&self) -> String {
        self.path.join("::")
    }

    /// Check if this is the same symbol (ignoring disambiguator)
    pub fn same_symbol(&self, other: &SymbolId) -> bool {
        self.kind == other.kind && self.path == other.path
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}({})", self.kind, self.qualified_name())?;
        if self.disambiguator > 0 {
            write!(f, "#{}", self.disambiguator)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_id_creation() {
        let struct_id = SymbolId::struct_id(vec!["api".to_string(), "User".to_string()]);
        assert_eq!(struct_id.kind, SymbolKind::Struct);
        assert_eq!(struct_id.path, vec!["api", "User"]);
        assert_eq!(struct_id.disambiguator, 0);
        assert_eq!(struct_id.name(), Some("User"));
        assert_eq!(struct_id.qualified_name(), "api::User");
    }

    #[test]
    fn test_symbol_id_display() {
        let struct_id = SymbolId::struct_id(vec!["api".to_string(), "User".to_string()]);
        assert_eq!(format!("{}", struct_id), "Struct(api::User)");

        let disambiguated = SymbolId::with_disambiguator(
            SymbolKind::Struct,
            vec!["api".to_string(), "User".to_string()],
            1,
        );
        assert_eq!(format!("{}", disambiguated), "Struct(api::User)#1");
    }

    #[test]
    fn test_same_symbol() {
        let id1 = SymbolId::struct_id(vec!["api".to_string(), "User".to_string()]);
        let id2 = SymbolId::with_disambiguator(
            SymbolKind::Struct,
            vec!["api".to_string(), "User".to_string()],
            1,
        );
        assert!(id1.same_symbol(&id2));
    }
}
