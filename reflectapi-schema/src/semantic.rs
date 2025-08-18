use crate::SymbolId;
/// Semantic Intermediate Representation for ReflectAPI
///
/// This module provides immutable, semantically-validated representations
/// of API schemas that have been processed through the normalization pipeline.
/// Unlike the raw schema types, these representations are guaranteed to be:
/// - Fully resolved (no dangling references)
/// - Semantically consistent (no conflicting definitions)
/// - Deterministically ordered (BTreeMap/BTreeSet for stable output)
use std::collections::{BTreeMap, BTreeSet};

/// Semantic schema with fully resolved types and deterministic ordering
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSchema {
    pub id: SymbolId,
    pub name: String,
    pub description: String,

    /// Functions ordered by SymbolId for deterministic output
    pub functions: BTreeMap<SymbolId, SemanticFunction>,

    /// All type definitions ordered by SymbolId
    pub types: BTreeMap<SymbolId, SemanticType>,

    /// Symbol table for efficient lookups
    pub symbol_table: SymbolTable,
}

/// Fully resolved function definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticFunction {
    pub id: SymbolId,
    pub name: String,
    pub path: String,
    pub description: String,
    pub deprecation_note: Option<String>,

    /// Resolved type references (no dangling pointers)
    pub input_type: Option<SymbolId>,
    pub input_headers: Option<SymbolId>,
    pub output_type: Option<SymbolId>,
    pub error_type: Option<SymbolId>,

    /// Metadata
    pub serialization: Vec<crate::SerializationMode>,
    pub readonly: bool,
    pub tags: BTreeSet<String>,
}

/// Resolved type definition with semantic validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticType {
    Primitive(SemanticPrimitive),
    Struct(SemanticStruct),
    Enum(SemanticEnum),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPrimitive {
    pub id: SymbolId,
    pub name: String,
    pub description: String,

    /// Resolved generic parameters
    pub parameters: Vec<SemanticTypeParameter>,

    /// Resolved fallback type reference
    pub fallback: Option<SymbolId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticStruct {
    pub id: SymbolId,
    pub name: String,
    pub serde_name: String,
    pub description: String,

    /// Resolved generic parameters
    pub parameters: Vec<SemanticTypeParameter>,

    /// Fields ordered deterministically
    pub fields: BTreeMap<SymbolId, SemanticField>,

    /// Semantic properties
    pub transparent: bool,
    pub is_tuple: bool,
    pub is_unit: bool,

    /// Language-specific configuration
    pub codegen_config: crate::LanguageSpecificTypeCodegenConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEnum {
    pub id: SymbolId,
    pub name: String,
    pub serde_name: String,
    pub description: String,

    /// Resolved generic parameters
    pub parameters: Vec<SemanticTypeParameter>,

    /// Variants ordered deterministically
    pub variants: BTreeMap<SymbolId, SemanticVariant>,

    /// Serde representation strategy
    pub representation: crate::Representation,

    /// Language-specific configuration
    pub codegen_config: crate::LanguageSpecificTypeCodegenConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticField {
    pub id: SymbolId,
    pub name: String,
    pub serde_name: String,
    pub description: String,
    pub deprecation_note: Option<String>,

    /// Resolved type reference
    pub type_ref: ResolvedTypeReference,

    /// Field properties
    pub required: bool,
    pub flattened: bool,

    /// Transform callback for custom processing
    pub transform_callback: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticVariant {
    pub id: SymbolId,
    pub name: String,
    pub serde_name: String,
    pub description: String,

    /// Fields ordered deterministically
    pub fields: BTreeMap<SymbolId, SemanticField>,

    /// Variant properties
    pub discriminant: Option<isize>,
    pub untagged: bool,
    pub field_style: FieldStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldStyle {
    Named,
    Unnamed,
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTypeParameter {
    pub name: String,
    pub description: String,

    /// Constraints on the type parameter
    pub bounds: Vec<SymbolId>,
    pub default: Option<SymbolId>,
}

/// Resolved type reference with guaranteed validity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypeReference {
    /// Target symbol (guaranteed to exist in symbol table)
    pub target: SymbolId,

    /// Resolved generic arguments
    pub arguments: Vec<ResolvedTypeReference>,

    /// Original type reference for debugging
    pub original_name: String,
}

/// Symbol table for efficient lookups and validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolTable {
    /// Map from SymbolId to symbol information
    pub symbols: BTreeMap<SymbolId, SymbolInfo>,

    /// Map from name path to SymbolId for lookups
    name_to_id: BTreeMap<Vec<String>, SymbolId>,

    /// Dependencies between symbols
    pub dependencies: BTreeMap<SymbolId, BTreeSet<SymbolId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    pub id: SymbolId,
    pub name: String,
    pub path: Vec<String>,
    pub kind: crate::SymbolKind,

    /// Whether this symbol is fully resolved
    pub resolved: bool,

    /// Dependencies of this symbol
    pub dependencies: BTreeSet<SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// Register a new symbol in the table
    pub fn register(&mut self, symbol: SymbolInfo) {
        let id = symbol.id.clone();
        let path = symbol.path.clone();

        self.symbols.insert(id.clone(), symbol);
        self.name_to_id.insert(path, id);
    }

    /// Lookup symbol by ID
    pub fn get(&self, id: &SymbolId) -> Option<&SymbolInfo> {
        self.symbols.get(id)
    }

    /// Lookup symbol by name path
    pub fn get_by_path(&self, path: &[String]) -> Option<&SymbolInfo> {
        self.name_to_id
            .get(path)
            .and_then(|id| self.symbols.get(id))
    }

    /// Get all symbols of a specific kind
    pub fn get_by_kind<'a>(
        &'a self,
        kind: &'a crate::SymbolKind,
    ) -> impl Iterator<Item = &SymbolInfo> + 'a {
        self.symbols.values().filter(move |info| &info.kind == kind)
    }

    /// Add dependency relationship
    pub fn add_dependency(&mut self, dependent: SymbolId, dependency: SymbolId) {
        self.dependencies
            .entry(dependent.clone())
            .or_default()
            .insert(dependency.clone());

        // Update symbol info
        if let Some(symbol) = self.symbols.get_mut(&dependent) {
            symbol.dependencies.insert(dependency);
        }
    }

    /// Get dependencies of a symbol
    pub fn get_dependencies(&self, id: &SymbolId) -> Option<&BTreeSet<SymbolId>> {
        self.dependencies.get(id)
    }

    /// Topological sort for dependency resolution
    pub fn topological_sort(&self) -> Result<Vec<SymbolId>, Vec<SymbolId>> {
        let mut visited = BTreeSet::new();
        let mut temp_visited = BTreeSet::new();
        let mut result = Vec::new();

        for id in self.symbols.keys() {
            if !visited.contains(id) {
                self.visit_topological(id, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        Ok(result)
    }

    fn visit_topological(
        &self,
        id: &SymbolId,
        visited: &mut BTreeSet<SymbolId>,
        temp_visited: &mut BTreeSet<SymbolId>,
        result: &mut Vec<SymbolId>,
    ) -> Result<(), Vec<SymbolId>> {
        if temp_visited.contains(id) {
            // Cycle detected
            return Err(vec![id.clone()]);
        }

        if visited.contains(id) {
            return Ok(());
        }

        temp_visited.insert(id.clone());

        if let Some(dependencies) = self.dependencies.get(id) {
            for dep in dependencies {
                self.visit_topological(dep, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(id);
        visited.insert(id.clone());
        result.push(id.clone());

        Ok(())
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticType {
    pub fn id(&self) -> &SymbolId {
        match self {
            SemanticType::Primitive(p) => &p.id,
            SemanticType::Struct(s) => &s.id,
            SemanticType::Enum(e) => &e.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            SemanticType::Primitive(p) => &p.name,
            SemanticType::Struct(s) => &s.name,
            SemanticType::Enum(e) => &e.name,
        }
    }
}

impl ResolvedTypeReference {
    /// Create a new resolved type reference
    pub fn new(
        target: SymbolId,
        arguments: Vec<ResolvedTypeReference>,
        original_name: String,
    ) -> Self {
        Self {
            target,
            arguments,
            original_name,
        }
    }

    /// Check if this is a primitive type reference
    pub fn is_primitive(&self, symbol_table: &SymbolTable) -> bool {
        symbol_table
            .get(&self.target)
            .map(|info| matches!(info.kind, crate::SymbolKind::Primitive))
            .unwrap_or(false)
    }

    /// Check if this is a generic type (has arguments)
    pub fn is_generic(&self) -> bool {
        !self.arguments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SymbolId, SymbolKind};

    #[test]
    fn test_symbol_table_basic_operations() {
        let mut table = SymbolTable::new();

        let user_id = SymbolId::struct_id(vec!["User".to_string()]);
        let user_info = SymbolInfo {
            id: user_id.clone(),
            name: "User".to_string(),
            path: vec!["User".to_string()],
            kind: SymbolKind::Struct,
            resolved: true,
            dependencies: BTreeSet::new(),
        };

        table.register(user_info);

        // Test lookup by ID
        assert!(table.get(&user_id).is_some());

        // Test lookup by path
        assert!(table.get_by_path(&["User".to_string()]).is_some());

        // Test lookup by kind
        let structs: Vec<_> = table.get_by_kind(&SymbolKind::Struct).collect();
        assert_eq!(structs.len(), 1);
    }

    #[test]
    fn test_symbol_table_dependencies() {
        let mut table = SymbolTable::new();

        let user_id = SymbolId::struct_id(vec!["User".to_string()]);
        let post_id = SymbolId::struct_id(vec!["Post".to_string()]);

        // Register symbols
        table.register(SymbolInfo {
            id: user_id.clone(),
            name: "User".to_string(),
            path: vec!["User".to_string()],
            kind: SymbolKind::Struct,
            resolved: true,
            dependencies: BTreeSet::new(),
        });

        table.register(SymbolInfo {
            id: post_id.clone(),
            name: "Post".to_string(),
            path: vec!["Post".to_string()],
            kind: SymbolKind::Struct,
            resolved: true,
            dependencies: BTreeSet::new(),
        });

        // Add dependency: Post depends on User
        table.add_dependency(post_id.clone(), user_id.clone());

        // Test dependency lookup
        let deps = table.get_dependencies(&post_id).unwrap();
        assert!(deps.contains(&user_id));

        // Test topological sort
        let sorted = table.topological_sort().unwrap();
        let user_pos = sorted.iter().position(|id| id == &user_id).unwrap();
        let post_pos = sorted.iter().position(|id| id == &post_id).unwrap();
        assert!(
            user_pos < post_pos,
            "User should come before Post in topological order"
        );
    }

    #[test]
    fn test_resolved_type_reference() {
        let string_id = SymbolId::new(SymbolKind::Primitive, vec!["String".to_string()]);
        let vec_id = SymbolId::new(SymbolKind::Struct, vec!["Vec".to_string()]);

        let string_ref =
            ResolvedTypeReference::new(string_id.clone(), vec![], "String".to_string());

        let vec_string_ref =
            ResolvedTypeReference::new(vec_id, vec![string_ref], "Vec<String>".to_string());

        assert!(!vec_string_ref.arguments.is_empty());
        assert!(vec_string_ref.is_generic());
        assert_eq!(vec_string_ref.arguments[0].target, string_id);
    }
}
