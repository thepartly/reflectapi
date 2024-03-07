mod spec;

use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub functions: Vec<Function>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<Type>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,

    #[serde(skip_serializing, default)]
    types_map: HashMap<String, usize>,
}

impl Display for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_json().as_str())
    }
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            name: String::new(),
            description: String::new(),
            functions: Vec::new(),
            types: Vec::new(),
            types_map: HashMap::new(),
            _debug: String::new(),
        }
    }

    pub fn types(&self) -> std::slice::Iter<Type> {
        self.types.iter()
    }

    pub fn reserve_type(&mut self, name: &str) -> bool {
        self.ensure_types_map();
        if self.types_map.contains_key(name) {
            return false;
        }
        self.types_map.insert(name.into(), usize::MAX);
        true
    }

    pub fn insert_type(&mut self, ty: Type) {
        self.ensure_types_map();
        if let Some(index) = self.types_map.get(ty.name()) {
            if index != &usize::MAX {
                return;
            }
        }
        self.types_map.insert(ty.name().into(), self.types.len());
        self.types.push(ty);
    }

    pub fn from_json(json: &str) -> Self {
        let mut result: Self = serde_json::from_str(json).unwrap();
        result.build_types_map();
        result
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn sort_types(&mut self) {
        self.types.sort_by(|a, b| a.name().cmp(&b.name()));
        self.build_types_map();
    }

    fn ensure_types_map(&mut self) {
        if self.types_map.is_empty() && !self.types.is_empty() {
            self.build_types_map();
        }
    }

    fn build_types_map(&mut self) {
        let mut types_map = HashMap::new();
        for (i, ty) in self.types().enumerate() {
            types_map.insert(ty.name().into(), i);
        }
        self.types_map = types_map;
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// Includes entity and action, for example: users.login
    pub name: String,
    /// Description of the call
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_type: Option<TypeReference>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub input_headers: std::collections::HashMap<HeaderName, TypeReference>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_type: Option<TypeReference>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub output_headers: std::collections::HashMap<HeaderName, TypeReference>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_type: Option<TypeReference>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub error_headers: std::collections::HashMap<HeaderName, TypeReference>,

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

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Function {
    pub fn new(name: String) -> Self {
        Function {
            name,
            description: String::new(),
            input_type: None,
            input_headers: std::collections::HashMap::new(),
            output_type: None,
            output_headers: std::collections::HashMap::new(),
            error_type: None,
            error_headers: std::collections::HashMap::new(),
            serialization: Vec::new(),
            _debug: String::new(),
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SerializationMode {
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
    pub parameters: Vec<TypeReference>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl TypeReference {
    pub fn new(name: String, parameters: Vec<TypeReference>) -> Self {
        TypeReference {
            name,
            parameters,
            _debug: String::new(),
        }
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeReference> {
        self.parameters.iter()
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

impl From<&str> for TypeReference {
    fn from(name: &str) -> Self {
        TypeReference {
            name: name.into(),
            parameters: Vec::new(),
            _debug: String::new(),
        }
    }
}

impl From<String> for TypeReference {
    fn from(name: String) -> Self {
        TypeReference {
            name,
            parameters: Vec::new(),
            _debug: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeParameter {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
}

impl TypeParameter {
    pub fn new(name: String) -> Self {
        TypeParameter {
            name,
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Type {
    Primitive(Primitive),
    Struct(Struct),
    Enum(Enum),
    Alias(Alias),
}

impl Type {
    pub fn name(&self) -> &str {
        match self {
            Type::Primitive(p) => &p.name,
            Type::Struct(s) => &s.name,
            Type::Enum(e) => &e.name,
            Type::Alias(a) => &a.name,
        }
    }

    pub fn rename(&mut self, new_name: String) {
        match self {
            Type::Primitive(p) => p.name = new_name,
            Type::Struct(s) => s.name = new_name,
            Type::Enum(e) => e.name = new_name,
            Type::Alias(a) => a.name = new_name,
        }
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn replace_type_references(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        match self {
            Type::Primitive(_) => {}
            Type::Struct(s) => s.replace_type_references(remap),
            Type::Enum(e) => e.replace_type_references(remap),
            Type::Alias(a) => a.replace_type_references(remap),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Primitive {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Primitive {
    pub fn new(name: String, description: String, parameters: Vec<TypeParameter>) -> Self {
        Primitive {
            name,
            description,
            parameters,
            _debug: String::new(),
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

impl Into<Type> for Primitive {
    fn into(self) -> Type {
        Type::Primitive(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<Field>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Struct {
    pub fn new(name: String) -> Self {
        Struct {
            name,
            description: String::new(),
            parameters: Vec::new(),
            fields: Vec::new(),
            _debug: String::new(),
        }
    }

    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    pub fn replace_type_references(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        for field in self.fields.iter_mut() {
            field.replace_type_reference(remap);
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

impl Into<Type> for Struct {
    fn into(self) -> Type {
        Type::Struct(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

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
    /// - Rust: Patch<T> (Patch is enum with Missing, None and Some variants)
    /// - TypeScript: T | null | undefined
    ///
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub required: bool,
    /// If serde flatten attribute is set on a field
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub flattened: bool,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Field {
    pub fn new(name: String, ty: TypeReference) -> Self {
        Field {
            name,
            description: String::new(),
            type_ref: ty,
            required: false,
            flattened: false,
            _debug: String::new(),
        }
    }

    pub fn replace_type_reference(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        if let Some(new_type_ref) = remap.get(&self.type_ref) {
            self.type_ref = new_type_ref.clone();
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "Representation::is_string", default)]
    pub representation: Representation,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub variants: Vec<Variant>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            description: String::new(),
            parameters: Vec::new(),
            representation: Representation::String,
            variants: Vec::new(),
            _debug: String::new(),
        }
    }

    pub fn variants(&self) -> std::slice::Iter<Variant> {
        self.variants.iter()
    }

    pub fn replace_type_references(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        for variant in self.variants.iter_mut() {
            variant.replace_type_references(remap);
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

impl Into<Type> for Enum {
    fn into(self) -> Type {
        Type::Enum(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variant {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discriminant: Option<i32>,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Variant {
    pub fn new(name: String) -> Self {
        Variant {
            name,
            description: String::new(),
            fields: Vec::new(),
            discriminant: None,
            _debug: String::new(),
        }
    }

    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    pub fn replace_type_references(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        for field in self.fields.iter_mut() {
            field.replace_type_reference(remap);
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Representation {
    /// Corresponds to untagged string only based representation
    String,

    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,

    /// Corresponsds to serde(untagged) attribute
    Untagged,
    /// Corresponsds to serde(tag = "...") attribute
    InnerTagged(String),
    /// Corresponsds to serde(tag = "...", content = "...") attribute
    OuterTagged {
        tag: String,
        content: String,
    },
}

impl Representation {
    fn is_string(&self) -> bool {
        matches!(self, Representation::String)
    }
}

impl Default for Representation {
    fn default() -> Self {
        Representation::String
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Alias {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    /// Aliased type
    #[serde(rename = "type")]
    pub type_ref: TypeReference,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    _debug: String,
}

impl Alias {
    pub fn new(name: String, ty: TypeReference) -> Self {
        Alias {
            name,
            description: String::new(),
            parameters: Vec::new(),
            type_ref: ty,
            _debug: String::new(),
        }
    }

    pub fn replace_type_references(
        &mut self,
        remap: &std::collections::HashMap<TypeReference, TypeReference>,
    ) {
        if let Some(new_type_reference) = remap.get(&self.type_ref) {
            self.type_ref = new_type_reference.clone();
        }
    }

    pub fn _debug(&mut self, debug: Option<String>) -> String {
        if let Some(debug) = debug {
            std::mem::replace(&mut self._debug, debug)
        } else {
            self._debug.clone()
        }
    }
}

impl Into<Type> for Alias {
    fn into(self) -> Type {
        Type::Alias(self)
    }
}

//
// Header name utility
//

/// Same as string but low case for content
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(String);

impl HeaderName {
    pub fn new(value: &str) -> Self {
        HeaderName(value.to_lowercase())
    }
}

impl std::fmt::Display for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for HeaderName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HeaderName::new(s))
    }
}

impl serde::Serialize for HeaderName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'a> serde::de::Deserialize<'a> for HeaderName {
    fn deserialize<D>(deserializer: D) -> Result<HeaderName, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(HeaderName::new(&s))
    }
}
