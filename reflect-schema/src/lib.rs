mod internal;

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schema {
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

impl Schema {
    pub fn new(name: String, description: String) -> Self {
        Schema {
            name,
            description: description,
            functions: Vec::new(),
            input_types: Typespace::new(),
            output_types: Typespace::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn functions(&self) -> &Vec<Function> {
        self.functions.as_ref()
    }

    pub fn input_types(&self) -> &Typespace {
        &self.input_types
    }

    pub fn output_types(&self) -> &Typespace {
        &self.output_types
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
                self.rename_input_type(&type_name, &type_name_parts.join("::"));

                let mut type_name_parts = type_name.split("::").collect::<Vec<_>>();
                type_name_parts.insert(type_name_parts.len() - 1, "output");
                self.rename_output_type(&type_name, &type_name_parts.join("::"));
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

    pub fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.rename_input_type(search_string, replacer);
        self.rename_output_type(search_string, replacer);
    }

    pub fn rename_input_type(&mut self, search_string: &str, replacer: &str) {
        self.input_types.rename_type(search_string, replacer);
        for function in self.functions.iter_mut() {
            function.rename_input_type(search_string, replacer);
        }
    }

    pub fn rename_output_type(&mut self, search_string: &str, replacer: &str) {
        self.output_types.rename_type(search_string, replacer);
        for function in self.functions.iter_mut() {
            function.rename_output_type(search_string, replacer);
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Typespace {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<Type>,

    #[serde(skip_serializing, default)]
    types_map: std::cell::RefCell<HashMap<String, usize>>,
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

    pub fn types(&self) -> std::slice::Iter<Type> {
        self.types.iter()
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.ensure_types_map();
        let index = {
            let b = self.types_map.borrow();
            b.get(name).map(|i| i.clone()).unwrap_or(usize::MAX)
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
            .map(|i| *i)
            .unwrap_or(usize::MAX);
        if index == usize::MAX {
            return None;
        }

        self.types_map.borrow_mut().remove(ty);
        Some(self.types.remove(index))
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        for ty in self.types.iter_mut() {
            ty.rename_type(search_string, replacer);
        }
        self.invalidate_types_map();
    }

    pub fn sort_types(&mut self) {
        self.types.sort_by(|a, b| a.name().cmp(&b.name()));
        self.build_types_map();
    }

    fn invalidate_types_map(&self) {
        *(self.types_map.borrow_mut()) = HashMap::new();
    }

    fn ensure_types_map(&self) {
        if self.types_map.borrow().is_empty() && !self.types.is_empty() {
            self.build_types_map();
        }
    }

    fn build_types_map(&self) {
        let mut types_map = HashMap::new();
        for (i, ty) in self.types().enumerate() {
            types_map.insert(ty.name().into(), i);
        }
        *(self.types_map.borrow_mut()) = types_map;
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
}

impl Function {
    pub fn new(name: String) -> Self {
        Function {
            name,
            description: String::new(),
            input_type: None,
            input_headers: None,
            output_type: None,
            error_type: None,
            serialization: Vec::new(),
            readonly: false,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    fn rename_input_type(&mut self, search_string: &str, replacer: &str) {
        if let Some(input_type) = &mut self.input_type {
            input_type.rename_type(search_string, replacer);
        }
        if let Some(input_headers) = &mut self.input_headers {
            input_headers.rename_type(search_string, replacer);
        }
    }

    fn rename_output_type(&mut self, search_string: &str, replacer: &str) {
        if let Some(output_type) = &mut self.output_type {
            output_type.rename_type(search_string, replacer);
        }
        if let Some(error_type) = &mut self.error_type {
            error_type.rename_type(search_string, replacer);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

impl TypeReference {
    pub fn new(name: String, parameters: Vec<TypeReference>) -> Self {
        TypeReference {
            name: name,
            parameters,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeReference> {
        self.parameters.iter()
    }

    pub fn fallback_recursively(&mut self, schema: &Typespace) {
        loop {
            let Some(type_def) = schema.get_type(self.name()) else {
                return;
            };
            if !type_def.fallback_internal(self) {
                return;
            }
        }
    }

    pub fn fallback_once(&mut self, schema: &Typespace) -> bool {
        let Some(type_def) = schema.get_type(self.name()) else {
            return false;
        };
        type_def.fallback_internal(self)
    }

    pub fn fallback_until<F>(&mut self, schema: &Schema, cond: F)
    where
        F: Fn(&TypeReference) -> bool,
    {
        loop {
            let Some(type_def) = schema.get_type(self.name()) else {
                return;
            };
            if !type_def.fallback_internal(self) || cond(self) {
                return;
            }
        }
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_ident(&self.name, search_string, replacer);
        for param in self.parameters.iter_mut() {
            param.rename_type(search_string, replacer);
        }
    }
}

impl From<&str> for TypeReference {
    fn from(name: &str) -> Self {
        TypeReference::new(name.into(), Vec::new())
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

    pub fn description(&self) -> &str {
        match self {
            Type::Primitive(p) => &p.description,
            Type::Struct(s) => &s.description,
            Type::Enum(e) => &e.description,
        }
    }

    pub fn set_description(&mut self, description: String) {
        match self {
            Type::Primitive(p) => p.description = description,
            Type::Struct(s) => s.description = description,
            Type::Enum(e) => e.description = description,
        }
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        match self {
            Type::Primitive(p) => p.parameters(),
            Type::Struct(s) => s.parameters(),
            Type::Enum(e) => e.parameters(),
        }
    }

    pub fn fallback(&self) -> Option<&TypeReference> {
        match self {
            Type::Primitive(p) => p.fallback.as_ref(),
            Type::Struct(_) => None,
            Type::Enum(_) => None,
        }
    }

    fn fallback_internal(&self, origin: &mut TypeReference) -> bool {
        match self {
            Type::Primitive(p) => p.fallback_internal(origin),
            Type::Struct(_) => false,
            Type::Enum(_) => false,
        }
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        match self {
            Type::Primitive(p) => p.rename_type(search_string, replacer),
            Type::Struct(s) => s.rename_type(search_string, replacer),
            Type::Enum(e) => e.rename_type(search_string, replacer),
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
            name,
            description,
            parameters,
            fallback,
        }
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        self.parameters.iter()
    }

    fn fallback_internal(&self, origin: &mut TypeReference) -> bool {
        // example:
        // Self is DashMap<K, V>
        // fallback is HashSet<V> (stupid example, but it demos generic param discard)
        // origin is DashMap<String, u8>
        // It should transform origin to HashSet<u8>
        let Some(fallback) = &self.fallback else {
            return false;
        };

        if let Some((type_def_param_index, _)) = self
            .parameters()
            .enumerate()
            .find(|(_, type_def_param)| type_def_param.name() == fallback.name())
        {
            // this is the case when fallback is to one of the generic parameters
            // for example, Arc<T> to T
            let Some(origin_type_ref_param) = origin.parameters.get(type_def_param_index) else {
                // It means the origin type reference does no provide correct number of generic parameters
                // required by the type definition
                // It is invalid schema
                return false;
            };
            origin.name = origin_type_ref_param.name.clone();
            origin.parameters = origin_type_ref_param.parameters.clone();
            return true;
        }

        let mut new_parameters_for_origin = Vec::new();
        let mut needs_parameters_rebuild = fallback.parameters().len() != origin.parameters.len();
        for (fallback_type_ref_param_index, fallback_type_ref_param) in
            fallback.parameters().enumerate()
        {
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
            let Some(origin_type_ref_param) = origin.parameters.get(type_def_param_index) else {
                // It means the origin type reference does no provide correct number of generic parameters
                // required by the type definition
                // It is invalid schema
                return false;
            };
            if type_def_param_index != fallback_type_ref_param_index {
                needs_parameters_rebuild = true;
            }
            new_parameters_for_origin.push(origin_type_ref_param);
        }

        origin.name = fallback.name.clone();
        if needs_parameters_rebuild {
            origin.parameters = new_parameters_for_origin
                .iter()
                .map(|i| std::ops::Deref::deref(i).clone())
                .collect();
        }
        true
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_ident(&self.name, search_string, replacer);
    }
}

impl Into<Type> for Primitive {
    fn into(self) -> Type {
        Type::Primitive(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Struct {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<Field>,

    /// If serde transparent attribute is set on a struct
    #[serde(skip_serializing_if = "is_false", default)]
    pub transparent: bool,
}

impl Struct {
    pub fn new(name: String) -> Self {
        Struct {
            name,
            description: String::new(),
            parameters: Vec::new(),
            fields: Vec::new(),
            transparent: false,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        self.parameters.iter()
    }

    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_ident(&self.name, search_string, replacer);
        for field in self.fields.iter_mut() {
            field.rename_type(search_string, replacer);
        }
    }
}

impl Into<Type> for Struct {
    fn into(self) -> Type {
        Type::Struct(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
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

    #[serde(skip, default)]
    pub transform_callback: String,
    #[serde(skip, default)]
    pub transform_callback_fn: Option<fn(&mut TypeReference, &Typespace) -> ()>,
}

impl Field {
    pub fn new(name: String, ty: TypeReference) -> Self {
        Field {
            name,
            description: String::new(),
            type_ref: ty,
            required: false,
            flattened: false,
            transform_callback: String::new(),
            transform_callback_fn: None,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.type_ref.rename_type(search_string, replacer);
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Enum {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    /// Generic type parameters, if any
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<TypeParameter>,

    #[serde(skip_serializing_if = "Representation::is_default", default)]
    pub representation: Representation,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub variants: Vec<Variant>,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            description: String::new(),
            parameters: Vec::new(),
            representation: Default::default(),
            variants: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        self.parameters.iter()
    }

    pub fn variants(&self) -> std::slice::Iter<Variant> {
        self.variants.iter()
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_ident(&self.name, search_string, replacer);
        for variant in self.variants.iter_mut() {
            variant.rename_type(search_string, replacer);
        }
    }
}

impl Into<Type> for Enum {
    fn into(self) -> Type {
        Type::Enum(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Variant {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discriminant: Option<isize>,

    /// If serde `untagged` attribute is set on a variant
    #[serde(skip_serializing_if = "is_false", default)]
    pub untagged: bool,
}

impl Variant {
    pub fn new(name: String) -> Self {
        Variant {
            name,
            description: String::new(),
            fields: Vec::new(),
            discriminant: None,
            untagged: false,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        for field in self.fields.iter_mut() {
            field.rename_type(search_string, replacer);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub enum Representation {
    /// The default.
    ///
    /// ```json
    /// {"variant1": {"key1": "value1", "key2": "value2"}}
    /// ```
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
    fn is_default(&self) -> bool {
        matches!(self, Representation::External)
    }
}

impl Default for Representation {
    fn default() -> Self {
        Representation::External
    }
}

fn rename_ident(name: &str, search_string: &str, replacer: &str) -> String {
    if search_string.ends_with("::") {
        // replacing module name
        if name.starts_with(search_string) {
            format!("{}{}", replacer, &name[search_string.len()..])
        } else {
            name.into()
        }
    } else {
        // replacing type name
        if name == search_string {
            replacer.into()
        } else {
            name.into()
        }
    }
}
