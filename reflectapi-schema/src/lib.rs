mod codegen;
mod ids;
mod internal;
mod normalize;
mod rename;
mod semantic;
mod subst;
mod symbol;
mod visit;

pub use self::codegen::*;
pub use self::ids::ensure_symbol_ids;
pub use self::normalize::{
    NamingResolutionStage, NormalizationError, NormalizationPipeline, NormalizationStage,
    Normalizer, TypeConsolidationStage,
};
pub use self::semantic::*;
pub use self::subst::{mk_subst, Instantiate, Substitute};
pub use self::symbol::{SymbolId, SymbolKind};
pub use self::visit::{VisitMut, Visitor};

#[cfg(feature = "glob")]
pub use self::rename::Glob;

#[cfg(feature = "glob")]
pub use glob::PatternError;

pub use self::rename::*;
use core::fmt;
use std::collections::BTreeSet;
use std::{
    collections::HashMap,
    ops::{ControlFlow, Index},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    /// Stable identifier for this schema
    #[serde(default)]
    pub id: SymbolId,

    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub functions: Vec<Function>,

    #[serde(skip_serializing_if = "Typespace::is_empty", default)]
    pub input_types: Typespace,

    #[serde(skip_serializing_if = "Typespace::is_empty", default)]
    pub output_types: Typespace,
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            id: SymbolId::new(SymbolKind::Struct, vec!["Schema".to_string()]),
            name: String::new(),
            description: String::new(),
            functions: Vec::new(),
            input_types: Typespace::new(),
            output_types: Typespace::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn functions(&self) -> std::slice::Iter<'_, Function> {
        self.functions.iter()
    }

    pub fn input_types(&self) -> &Typespace {
        &self.input_types
    }

    pub fn is_input_type(&self, name: &str) -> bool {
        self.input_types.has_type(name)
    }

    pub fn output_types(&self) -> &Typespace {
        &self.output_types
    }

    pub fn is_output_type(&self, name: &str) -> bool {
        self.output_types.has_type(name)
    }

    pub fn extend(&mut self, other: Self) {
        let Self {
            functions,
            input_types,
            output_types,
            name: _,
            description: _,
            id: _,
        } = other;
        self.functions.extend(functions);
        self.input_types.extend(input_types);
        self.output_types.extend(output_types);
    }

    pub fn prepend_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        for function in self.functions.iter_mut() {
            function.path = format!("{}{}", path, function.path);
        }
    }

    pub fn consolidate_types(&mut self) -> Vec<String> {
        // this is probably very inefficient approach to deduplicate types
        // but is simple enough and will work for foreseeable future
        loop {
            let mut all_types = std::collections::HashSet::new();
            let mut colliding_types = std::collections::HashSet::new();
            let mut colliging_non_equal_types = std::collections::HashSet::new();

            for input_type in self.input_types.types() {
                all_types.insert(input_type.name().to_string());
                if let Some(output_type) = self.output_types.get_type(input_type.name()) {
                    colliding_types.insert(input_type.name().to_string());
                    if input_type != output_type {
                        colliging_non_equal_types.insert(input_type.name().to_string());
                    }
                }
            }
            for output_type in self.output_types.types() {
                all_types.insert(output_type.name().to_string());
                if let Some(input_type) = self.input_types.get_type(output_type.name()) {
                    colliding_types.insert(output_type.name().to_string());
                    if input_type != output_type {
                        colliging_non_equal_types.insert(output_type.name().to_string());
                    }
                }
            }

            if colliging_non_equal_types.is_empty() {
                let mut r: Vec<_> = all_types.into_iter().collect();
                r.sort();
                return r;
            }

            for type_name in colliging_non_equal_types.iter() {
                // we assume for now that there is not collision with input / output submodule

                let mut type_name_parts = type_name.split("::").collect::<Vec<_>>();
                type_name_parts.insert(type_name_parts.len() - 1, "input");
                self.rename_input_types(type_name.as_str(), &type_name_parts.join("::"));

                let mut type_name_parts = type_name.split("::").collect::<Vec<_>>();
                type_name_parts.insert(type_name_parts.len() - 1, "output");
                self.rename_output_types(type_name.as_str(), &type_name_parts.join("::"));
            }
        }
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        if let Some(t) = self.input_types.get_type(name) {
            return Some(t);
        }
        if let Some(t) = self.output_types.get_type(name) {
            return Some(t);
        }
        None
    }

    #[cfg(feature = "glob")]
    pub fn glob_rename_types(
        &mut self,
        glob: &str,
        replacer: &str,
    ) -> Result<(), glob::PatternError> {
        let pattern = glob.parse::<Glob>()?;
        self.rename_types(&pattern, replacer);
        Ok(())
    }

    pub fn rename_types(&mut self, pattern: impl Pattern, replacer: &str) -> usize {
        self.rename_input_types(pattern, replacer) + self.rename_output_types(pattern, replacer)
    }

    fn rename_input_types(&mut self, pattern: impl Pattern, replacer: &str) -> usize {
        match Renamer::new(pattern, replacer).visit_schema_inputs(self) {
            ControlFlow::Continue(c) | ControlFlow::Break(c) => c,
        }
    }

    fn rename_output_types(&mut self, pattern: impl Pattern, replacer: &str) -> usize {
        match Renamer::new(pattern, replacer).visit_schema_outputs(self) {
            ControlFlow::Continue(c) | ControlFlow::Break(c) => c,
        }
    }

    pub fn fold_transparent_types(&mut self) {
        // Replace the transparent struct `strukt` with it's single field.
        #[derive(Debug)]
        struct SubstVisitor {
            strukt: Struct,
            to: TypeReference,
        }

        impl SubstVisitor {
            fn new(strukt: Struct) -> Self {
                assert!(strukt.transparent && strukt.fields.len() == 1);
                Self {
                    to: strukt.fields[0].type_ref.clone(),
                    strukt,
                }
            }
        }

        impl Visitor for SubstVisitor {
            type Output = ();

            fn visit_type_ref(
                &mut self,
                type_ref: &mut TypeReference,
            ) -> ControlFlow<Self::Output, Self::Output> {
                if type_ref.name == self.strukt.name {
                    let subst = subst::mk_subst(&self.strukt.parameters, &type_ref.arguments);
                    *type_ref = self.to.clone().subst(&subst);
                }

                type_ref.visit_mut(self)?;

                ControlFlow::Continue(())
            }
        }

        let transparent_types = self
            .input_types()
            .types()
            .filter_map(|t| {
                t.as_struct()
                    .filter(|i| i.transparent && i.fields.len() == 1)
                    .cloned()
            })
            .collect::<Vec<_>>();

        for strukt in transparent_types {
            self.input_types.remove_type(strukt.name());
            SubstVisitor::new(strukt).visit_schema_inputs(self);
        }

        let transparent_types = self
            .output_types()
            .types()
            .filter_map(|t| {
                t.as_struct()
                    .filter(|i| i.transparent && i.fields.len() == 1)
                    .cloned()
            })
            .collect::<Vec<_>>();

        for strukt in transparent_types {
            self.output_types.remove_type(strukt.name());
            SubstVisitor::new(strukt).visit_schema_outputs(self);
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Typespace {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    types: Vec<Type>,

    #[serde(skip_serializing, default)]
    types_map: std::cell::RefCell<HashMap<String, usize>>,
}

impl fmt::Debug for Typespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.types.iter().map(|t| (t.name().to_string(), t)))
            .finish()
    }
}

impl Typespace {
    pub fn new() -> Self {
        Typespace {
            types: Vec::new(),
            types_map: std::cell::RefCell::new(HashMap::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn types(&self) -> std::slice::Iter<'_, Type> {
        self.types.iter()
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.ensure_types_map();
        let index = {
            let b = self.types_map.borrow();
            b.get(name).copied().unwrap_or(usize::MAX)
        };
        if index == usize::MAX {
            return None;
        }
        self.types.get(index)
    }

    pub fn reserve_type(&mut self, name: &str) -> bool {
        self.ensure_types_map();
        if self.types_map.borrow().contains_key(name) {
            return false;
        }
        self.types_map.borrow_mut().insert(name.into(), usize::MAX);
        true
    }

    pub fn insert_type(&mut self, ty: Type) {
        self.ensure_types_map();
        if let Some(index) = self.types_map.borrow().get(ty.name()) {
            if index != &usize::MAX {
                return;
            }
        }
        self.types_map
            .borrow_mut()
            .insert(ty.name().into(), self.types.len());
        self.types.push(ty);
    }

    pub fn remove_type(&mut self, ty: &str) -> Option<Type> {
        self.ensure_types_map();
        let index = self
            .types_map
            .borrow()
            .get(ty)
            .copied()
            .unwrap_or(usize::MAX);
        if index == usize::MAX {
            return None;
        }

        self.types_map.borrow_mut().remove(ty);
        Some(self.types.remove(index))
    }

    pub fn sort_types(&mut self) {
        self.types.sort_by(|a, b| a.name().cmp(b.name()));
        self.build_types_map();
    }

    pub fn has_type(&self, name: &str) -> bool {
        self.ensure_types_map();
        self.types_map.borrow().contains_key(name)
    }

    pub fn extend(&mut self, other: Self) {
        self.ensure_types_map();
        for ty in other.types {
            if self.has_type(ty.name()) {
                continue;
            }
            self.insert_type(ty);
        }
    }

    fn invalidate_types_map(&self) {
        self.types_map.borrow_mut().clear()
    }

    fn ensure_types_map(&self) {
        if self.types_map.borrow().is_empty() && !self.types.is_empty() {
            self.build_types_map();
        }
    }

    fn build_types_map(&self) {
        let mut types_map = HashMap::new();
        for (i, ty) in self.types.iter().enumerate() {
            types_map.insert(ty.name().into(), i);
        }
        *(self.types_map.borrow_mut()) = types_map;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// Stable identifier for this function
    #[serde(default)]
    pub id: SymbolId,

    /// Includes entity and action, for example: users.login
    pub name: String,
    /// URL mounting path, for example: /api/v1
    pub path: String,
    /// Description of the call
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
    /// Deprecation note. If none, function is not deprecated.
    /// If present as empty string, function is deprecated without a note.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecation_note: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_type: Option<TypeReference>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_headers: Option<TypeReference>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_type: Option<TypeReference>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_type: Option<TypeReference>,

    ///
    /// Supported content types for request and response bodies.
    ///
    /// Note: serialization for header values is not affected by this field.
    /// For displayable types of fields, it is encoded in plain strings.
    /// For non-displayable types, it is encoded as json.
    ///
    /// Default: only json if empty
    ///
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub serialization: Vec<SerializationMode>,

    /// If a function is readonly, it means it does not modify the state of an application
    #[serde(skip_serializing_if = "is_false", default)]
    pub readonly: bool,

    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub tags: BTreeSet<String>,
}

impl Function {
    pub fn new(name: String) -> Self {
        Function {
            id: SymbolId::endpoint_id(vec![name.clone()]),
            name,
            deprecation_note: Default::default(),
            path: Default::default(),
            description: Default::default(),
            input_type: None,
            input_headers: None,
            output_type: None,
            error_type: None,
            serialization: Default::default(),
            readonly: Default::default(),
            tags: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn deprecated(&self) -> bool {
        self.deprecation_note.is_some()
    }

    pub fn input_type(&self) -> Option<&TypeReference> {
        self.input_type.as_ref()
    }

    pub fn input_headers(&self) -> Option<&TypeReference> {
        self.input_headers.as_ref()
    }

    pub fn output_type(&self) -> Option<&TypeReference> {
        self.output_type.as_ref()
    }

    pub fn error_type(&self) -> Option<&TypeReference> {
        self.error_type.as_ref()
    }

    pub fn serialization(&self) -> std::slice::Iter<'_, SerializationMode> {
        self.serialization.iter()
    }

    pub fn readonly(&self) -> bool {
        self.readonly
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializationMode {
    #[default]
    Json,
    Msgpack,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct TypeReference {
    pub name: String,
    /**
     * References to actual types to use instead of the type parameters
     * declared on the referred generic type
     */
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub arguments: Vec<TypeReference>,
}

impl TypeReference {
    pub fn new(name: impl Into<String>, arguments: Vec<TypeReference>) -> Self {
        TypeReference {
            name: name.into(),
            arguments,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn arguments(&self) -> std::slice::Iter<'_, TypeReference> {
        self.arguments.iter()
    }

    pub fn fallback_recursively(&mut self, schema: &Typespace) {
        loop {
            let Some(type_def) = schema.get_type(self.name()) else {
                return;
            };
            let Some(fallback_type_ref) = type_def.fallback_internal(self) else {
                return;
            };
            *self = fallback_type_ref;
        }
    }

    pub fn fallback_once(&self, schema: &Typespace) -> Option<TypeReference> {
        let type_def = schema.get_type(self.name())?;
        type_def.fallback_internal(self)
    }
}

impl From<&str> for TypeReference {
    fn from(name: &str) -> Self {
        TypeReference::new(name, Vec::new())
    }
}

impl From<String> for TypeReference {
    fn from(name: String) -> Self {
        TypeReference::new(name, Vec::new())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeParameter {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
}

impl TypeParameter {
    pub fn new(name: String, description: String) -> Self {
        TypeParameter { name, description }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl From<&str> for TypeParameter {
    fn from(name: &str) -> Self {
        TypeParameter {
            name: name.into(),
            description: String::new(),
        }
    }
}

impl From<String> for TypeParameter {
    fn from(name: String) -> Self {
        TypeParameter {
            name,
            description: String::new(),
        }
    }
}

impl PartialEq for TypeParameter {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TypeParameter {}

impl std::hash::Hash for TypeParameter {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Type {
    Primitive(Primitive),
    Struct(Struct),
    Enum(Enum),
}

impl Type {
    pub fn name(&self) -> &str {
        match self {
            Type::Primitive(p) => &p.name,
            Type::Struct(s) => &s.name,
            Type::Enum(e) => &e.name,
        }
    }

    pub fn serde_name(&self) -> &str {
        match self {
            Type::Primitive(_) => self.name(),
            Type::Struct(s) => s.serde_name(),
            Type::Enum(e) => e.serde_name(),
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Type::Primitive(p) => &p.description,
            Type::Struct(s) => &s.description,
            Type::Enum(e) => &e.description,
        }
    }

    pub fn parameters(&self) -> std::slice::Iter<'_, TypeParameter> {
        match self {
            Type::Primitive(p) => p.parameters(),
            Type::Struct(s) => s.parameters(),
            Type::Enum(e) => e.parameters(),
        }
    }

    pub fn as_struct(&self) -> Option<&Struct> {
        match self {
            Type::Struct(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, Type::Struct(_))
    }

    pub fn as_enum(&self) -> Option<&Enum> {
        match self {
            Type::Enum(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, Type::Enum(_))
    }

    pub fn as_primitive(&self) -> Option<&Primitive> {
        match self {
            Type::Primitive(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(self, Type::Primitive(_))
    }

    fn fallback_internal(&self, origin: &TypeReference) -> Option<TypeReference> {
        match self {
            Type::Primitive(p) => p.fallback_internal(origin),
            Type::Struct(_) => None,
            Type::Enum(_) => None,
        }
    }

    pub fn __internal_rename_current(&mut self, new_name: String) {
        match self {
            Type::Primitive(p) => p.name = new_name,
            Type::Struct(s) => s.name = new_name,
            Type::Enum(e) => e.name = new_name,
        }
    }

    pub fn __internal_rebind_generic_parameters(
        &mut self,
        unresolved_to_resolved_map: &std::collections::HashMap<TypeReference, TypeReference>,
        schema: &Typespace,
    ) {
        internal::replace_type_references_for_type(self, unresolved_to_resolved_map, schema)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Primitive {
    /// Stable identifier for this primitive type
    #[serde(default)]
    pub id: SymbolId,

    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    /// Fallback type to use when the type is not supported by the target language
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fallback: Option<TypeReference>,
}

impl Primitive {
    pub fn new(
        name: String,
        description: String,
        parameters: Vec<TypeParameter>,
        fallback: Option<TypeReference>,
    ) -> Self {
        Primitive {
            id: SymbolId::new(SymbolKind::Primitive, vec![name.clone()]),
            name,
            description,
            parameters,
            fallback,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<'_, TypeParameter> {
        self.parameters.iter()
    }

    pub fn fallback(&self) -> Option<&TypeReference> {
        self.fallback.as_ref()
    }

    fn fallback_internal(&self, origin: &TypeReference) -> Option<TypeReference> {
        // example:
        // Self is DashMap<K, V>
        // fallback is HashSet<V> (stupid example, but it demos generic param discard)
        // origin is DashMap<String, u8>
        // It should transform origin to HashSet<u8>
        let fallback = self.fallback.as_ref()?;

        if let Some((type_def_param_index, _)) = self
            .parameters()
            .enumerate()
            .find(|(_, type_def_param)| type_def_param.name() == fallback.name())
        {
            // this is the case when fallback is to one of the generic parameters
            // for example, Arc<T> to T
            let Some(origin_type_ref_param) = origin.arguments.get(type_def_param_index) else {
                // It means the origin type reference does no provide correct number of generic parameters
                // required by the type definition
                // It is invalid schema
                return None;
            };
            return Some(TypeReference {
                name: origin_type_ref_param.name.clone(),
                arguments: origin_type_ref_param.arguments.clone(),
            });
        }

        let mut new_arguments_for_origin = Vec::new();
        for fallback_type_ref_param in fallback.arguments() {
            let Some((type_def_param_index, _)) =
                self.parameters().enumerate().find(|(_, type_def_param)| {
                    type_def_param.name() == fallback_type_ref_param.name()
                })
            else {
                // It means fallback type does not have
                // as much generic parameters as this type definition
                // in our example, it would be index 0
                continue;
            };

            // in our example type_def_param_index would be index 1 for V
            let Some(origin_type_ref_param) = origin.arguments.get(type_def_param_index) else {
                // It means the origin type reference does no provide correct number of generic parameters
                // required by the type definition
                // It is invalid schema
                return None;
            };
            new_arguments_for_origin.push(origin_type_ref_param.clone());
        }

        Some(TypeReference {
            name: fallback.name.clone(),
            arguments: new_arguments_for_origin,
        })
    }
}

impl From<Primitive> for Type {
    fn from(val: Primitive) -> Self {
        Type::Primitive(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Struct {
    /// Stable identifier for this struct
    #[serde(default)]
    pub id: SymbolId,

    /// Name of a struct, should be a valid Rust struct name identifier
    pub name: String,

    /// If a serialized name is not a valid Rust struct name identifier
    /// then this defines the name of a struct to be used in serialization
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,

    /// Markdown docs for the struct
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    pub fields: Fields,

    /// If serde transparent attribute is set on a struct
    #[serde(skip_serializing_if = "is_false", default)]
    pub transparent: bool,

    #[serde(skip_serializing_if = "is_default", default)]
    pub codegen_config: LanguageSpecificTypeCodegenConfig,
}

impl Struct {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Struct {
            id: SymbolId::struct_id(vec![name.clone()]),
            name,
            serde_name: Default::default(),
            description: Default::default(),
            parameters: Default::default(),
            fields: Default::default(),
            transparent: Default::default(),
            codegen_config: Default::default(),
        }
    }

    /// Returns the name of a struct, should be a valid Rust struct name identifier
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the name of a struct to be used in serialization
    pub fn serde_name(&self) -> &str {
        if self.serde_name.is_empty() {
            self.name.as_str()
        } else {
            self.serde_name.as_str()
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<'_, TypeParameter> {
        self.parameters.iter()
    }

    pub fn fields(&self) -> std::slice::Iter<'_, Field> {
        self.fields.iter()
    }

    pub fn transparent(&self) -> bool {
        self.transparent
    }

    /// Returns true if a struct has 1 field and it is either named "0"
    /// or is transparent in the serialized form
    pub fn is_alias(&self) -> bool {
        self.fields.len() == 1 && (self.fields[0].name() == "0" || self.transparent)
    }

    /// Returns true is a struct is a Rust unit struct.
    /// Please note, that a unit struct is also an alias
    // NOTE(andy): does this function make sense? A unit struct is a struct with no fields.
    pub fn is_unit(&self) -> bool {
        let Some(first_field) = self.fields.iter().next() else {
            return false;
        };

        self.fields.len() == 1
            && first_field.name() == "0"
            && first_field.type_ref.name == "std::tuple::Tuple0"
            && !first_field.required
    }

    /// Returns true if a struct is a Rust tuple struct.
    pub fn is_tuple(&self) -> bool {
        !self.fields.is_empty()
            && self
                .fields
                .iter()
                .all(|f| f.name().parse::<usize>().is_ok())
    }
}

impl From<Struct> for Type {
    fn from(val: Struct) -> Self {
        Type::Struct(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Fields {
    /// Named struct or variant:
    /// struct S { a: u8, b: u8 }
    /// enum S { T { a: u8, b: u8 } }
    Named(Vec<Field>),
    /// Tuple struct or variant:
    /// struct S(u8, u8);
    /// enum S { T(u8, u8) }
    Unnamed(Vec<Field>),
    /// Unit struct or variant:
    ///
    /// struct S;
    /// enum S { U }
    #[default]
    None,
}

impl Fields {
    pub fn is_empty(&self) -> bool {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => fields.is_empty(),
            Fields::None => true,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => fields.len(),
            Fields::None => 0,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Field> {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => fields.iter(),
            Fields::None => [].iter(),
        }
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Field> {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => fields.iter_mut(),
            Fields::None => [].iter_mut(),
        }
    }
}

impl Index<usize> for Fields {
    type Output = Field;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => &fields[index],
            Fields::None => panic!("index out of bounds"),
        }
    }
}

impl IntoIterator for Fields {
    type Item = Field;
    type IntoIter = std::vec::IntoIter<Field>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Fields::Named(fields) => fields.into_iter(),
            Fields::Unnamed(fields) => fields.into_iter(),
            Fields::None => vec![].into_iter(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Field {
    /// Stable identifier for this field
    #[serde(default)]
    pub id: SymbolId,

    /// Field name, should be a valid Rust field name identifier
    pub name: String,
    /// If a serialized name is not a valid Rust field name identifier
    /// then this defines the name of a field to be used in serialization
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
    /// Rust docs for the field
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Deprecation note. If none, field is not deprecated.
    /// If present as empty string, field is deprecated without a note.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecation_note: Option<String>,

    /// Type of a field
    #[serde(rename = "type")]
    pub type_ref: TypeReference,
    /// required and not nullable:
    /// - field always present and not null / none
    ///
    /// required and nullable:
    /// - Rust: Option<T>, do not skip serializing if None
    /// - TypeScript: T | null, do not skip serializing if null
    ///
    /// not required and not nullable:
    /// - Rust: Option<T>, skip serializing if None
    /// - TypeScript: T | undefined, skip serializing if undefined
    ///
    /// not required and nullable:
    ///   serializers and deserializers are required to differentiate between
    ///   missing fields and null / none fields
    /// - Rust: reflectapi::Option<T> is enum with Undefined, None and Some variants
    /// - TypeScript: T | null | undefined
    ///
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub required: bool,
    /// If serde flatten attribute is set on a field
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub flattened: bool,

    #[serde(skip, default)]
    pub transform_callback: String,
    #[serde(skip, default)]
    pub transform_callback_fn: Option<fn(&mut TypeReference, &Typespace) -> ()>,
}

impl Field {
    pub fn new(name: String, type_ref: TypeReference) -> Self {
        Field {
            id: SymbolId::field_id(vec![], name.clone()),
            name,
            type_ref,
            serde_name: Default::default(),
            description: Default::default(),
            deprecation_note: Default::default(),
            required: Default::default(),
            flattened: Default::default(),
            transform_callback: Default::default(),
            transform_callback_fn: Default::default(),
        }
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn is_named(&self) -> bool {
        !self.is_unnamed()
    }

    pub fn is_unnamed(&self) -> bool {
        self.name.parse::<u64>().is_ok()
    }

    pub fn serde_name(&self) -> &str {
        if self.serde_name.is_empty() {
            self.name.as_str()
        } else {
            self.serde_name.as_str()
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn deprecated(&self) -> bool {
        self.deprecation_note.is_some()
    }

    pub fn type_ref(&self) -> &TypeReference {
        &self.type_ref
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn flattened(&self) -> bool {
        self.flattened
    }

    pub fn transform_callback(&self) -> &str {
        self.transform_callback.as_str()
    }

    pub fn transform_callback_fn(&self) -> Option<fn(&mut TypeReference, &Typespace)> {
        self.transform_callback_fn
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == Default::default()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Enum {
    /// Stable identifier for this enum
    #[serde(default)]
    pub id: SymbolId,

    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "Representation::is_default", default)]
    pub representation: Representation,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub variants: Vec<Variant>,

    #[serde(skip_serializing_if = "is_default", default)]
    pub codegen_config: LanguageSpecificTypeCodegenConfig,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            id: SymbolId::enum_id(vec![name.clone()]),
            name,
            serde_name: Default::default(),
            description: Default::default(),
            parameters: Default::default(),
            representation: Default::default(),
            variants: Default::default(),
            codegen_config: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn serde_name(&self) -> &str {
        if self.serde_name.is_empty() {
            self.name.as_str()
        } else {
            self.serde_name.as_str()
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<'_, TypeParameter> {
        self.parameters.iter()
    }

    pub fn representation(&self) -> &Representation {
        &self.representation
    }

    pub fn variants(&self) -> std::slice::Iter<'_, Variant> {
        self.variants.iter()
    }
}

impl From<Enum> for Type {
    fn from(val: Enum) -> Self {
        Type::Enum(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Variant {
    /// Stable identifier for this variant
    #[serde(default)]
    pub id: SymbolId,

    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    pub fields: Fields,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discriminant: Option<isize>,

    /// If serde `untagged` attribute is set on a variant
    #[serde(skip_serializing_if = "is_false", default)]
    pub untagged: bool,
}

impl Variant {
    pub fn new(name: String) -> Self {
        Variant {
            id: SymbolId::variant_id(vec![], name.clone()),
            name,
            serde_name: String::new(),
            description: String::new(),
            fields: Fields::None,
            discriminant: None,
            untagged: false,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn serde_name(&self) -> &str {
        if self.serde_name.is_empty() {
            self.name.as_str()
        } else {
            self.serde_name.as_str()
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn fields(&self) -> std::slice::Iter<'_, Field> {
        self.fields.iter()
    }

    pub fn discriminant(&self) -> Option<isize> {
        self.discriminant
    }

    pub fn untagged(&self) -> bool {
        self.untagged
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    /// The default.
    ///
    /// ```json
    /// {"variant1": {"key1": "value1", "key2": "value2"}}
    /// ```
    #[default]
    External,

    /// `#[serde(tag = "type")]`
    ///
    /// ```json
    /// {"type": "variant1", "key1": "value1", "key2": "value2"}
    /// ```
    Internal { tag: String },

    /// `#[serde(tag = "t", content = "c")]`
    ///
    /// ```json
    /// {"t": "variant1", "c": {"key1": "value1", "key2": "value2"}}
    /// ```
    Adjacent { tag: String, content: String },

    /// `#[serde(untagged)]`
    ///
    /// ```json
    /// {"key1": "value1", "key2": "value2"}
    /// ```
    None,
}

impl Representation {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_default(&self) -> bool {
        matches!(self, Representation::External)
    }

    pub fn is_external(&self) -> bool {
        matches!(self, Representation::External)
    }

    pub fn is_internal(&self) -> bool {
        matches!(self, Representation::Internal { .. })
    }

    pub fn is_adjacent(&self) -> bool {
        matches!(self, Representation::Adjacent { .. })
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Representation::None)
    }
}
