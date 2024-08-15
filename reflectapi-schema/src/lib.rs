mod internal;

// #[cfg(feature = "openapi")]
pub mod openapi;

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

type Subst = HashMap<String, TypeReference>;

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Schema {
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

    pub fn functions(&self) -> std::slice::Iter<Function> {
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
            ..
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
                self.rename_input_type(type_name, &type_name_parts.join("::"));

                let mut type_name_parts = type_name.split("::").collect::<Vec<_>>();
                type_name_parts.insert(type_name_parts.len() - 1, "output");
                self.rename_output_type(type_name, &type_name_parts.join("::"));
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

    pub fn fold_transparent_types(&mut self) {
        let transparent_types = self
            .input_types()
            .types()
            .filter_map(|t| {
                t.as_struct()
                    .filter(|i| i.transparent && i.fields.len() == 1)
                    .map(|s| (s.name.clone(), s.fields[0].type_ref.name.clone()))
            })
            .collect::<Vec<_>>();
        for (from, to) in transparent_types {
            self.input_types.remove_type(&from);
            self.rename_input_type(&from, &to);
        }

        let transparent_types = self
            .output_types()
            .types()
            .filter_map(|t| {
                t.as_struct()
                    .filter(|i| i.transparent && i.fields.len() == 1)
                    .map(|s| (s.name.clone(), s.fields[0].type_ref.name.clone()))
            })
            .collect::<Vec<_>>();
        for (from, to) in transparent_types {
            self.output_types.remove_type(&from);
            self.rename_output_type(&from, &to);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Typespace {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    types: Vec<Type>,

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

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        for ty in self.types.iter_mut() {
            ty.rename_type(search_string, replacer);
        }
        self.invalidate_types_map();
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
        *(self.types_map.borrow_mut()) = HashMap::new();
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
    /// Includes entity and action, for example: users.login
    pub name: String,
    /// URL mounting path, for example: /api/v1
    pub path: String,
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
            path: String::new(),
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

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
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

    pub fn serialization(&self) -> std::slice::Iter<SerializationMode> {
        self.serialization.iter()
    }

    pub fn readonly(&self) -> bool {
        self.readonly
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

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn new(name: String, arguments: Vec<TypeReference>) -> Self {
        TypeReference { name, arguments }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn arguments(&self) -> std::slice::Iter<TypeReference> {
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

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_type_or_module(&self.name, search_string, replacer);
        for param in self.arguments.iter_mut() {
            param.rename_type(search_string, replacer);
        }
    }

    fn instantiate(self, subst: &Subst) -> TypeReference {
        match subst.get(&self.name) {
            Some(ty) => {
                assert!(
                    self.arguments.is_empty(),
                    "type parameter cannot have type arguments (no HKTs)"
                );
                ty.clone()
            }
            None => Self {
                name: self.name,
                arguments: self
                    .arguments
                    .into_iter()
                    .map(|a| a.instantiate(subst))
                    .collect(),
            },
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

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
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

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
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

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_type_or_module(&self.name, search_string, replacer);
    }
}

impl From<Primitive> for Type {
    fn from(val: Primitive) -> Self {
        Type::Primitive(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Struct {
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
            serde_name: String::new(),
            description: String::new(),
            parameters: Vec::new(),
            fields: Vec::new(),
            transparent: false,
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

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        self.parameters.iter()
    }

    pub fn fields(&self) -> std::slice::Iter<Field> {
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
    pub fn is_unit(&self) -> bool {
        let Some(first_field) = self.fields.first() else {
            return false;
        };

        self.fields.len() == 1
            && first_field.name() == "0"
            && first_field.type_ref.name == "unit"
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

    /// Return a new `Struct` with each type parameter substituted with a type
    pub(crate) fn instantiate(self, type_args: &[TypeReference]) -> Self {
        assert_eq!(
            self.parameters.len(),
            type_args.len(),
            "expected {} type arguments, got {}",
            self.parameters.len(),
            type_args.len()
        );

        let subst = self
            .parameters
            .iter()
            .map(|p| p.name.to_owned())
            .zip(type_args.iter().cloned())
            .collect::<HashMap<_, _>>();

        Self {
            parameters: vec![],
            fields: self
                .fields
                .into_iter()
                .map(|f| f.instantiate(&subst))
                .collect(),
            ..self
        }
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_type_or_module(&self.name, search_string, replacer);
        for field in self.fields.iter_mut() {
            field.rename_type(search_string, replacer);
        }
    }
}

impl From<Struct> for Type {
    fn from(val: Struct) -> Self {
        Type::Struct(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Field {
    /// Field name, should be a valid Rust field name identifier
    pub name: String,
    /// If a serialized name is not a valid Rust field name identifier
    /// then this defines the name of a field to be used in serialization
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
    /// Rust docs for the field
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
    pub fn new(name: String, ty: TypeReference) -> Self {
        Field {
            name,
            serde_name: String::new(),
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

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.type_ref.rename_type(search_string, replacer);
    }

    fn instantiate(self, subst: &Subst) -> Field {
        Self {
            type_ref: self.type_ref.instantiate(subst),
            ..self
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Enum {
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
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            serde_name: String::new(),
            description: String::new(),
            parameters: Vec::new(),
            representation: Default::default(),
            variants: Vec::new(),
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

    pub fn parameters(&self) -> std::slice::Iter<TypeParameter> {
        self.parameters.iter()
    }

    pub fn representation(&self) -> &Representation {
        &self.representation
    }

    pub fn variants(&self) -> std::slice::Iter<Variant> {
        self.variants.iter()
    }

    /// Return a new `Enum` with each type parameter substituted with a type
    pub(crate) fn instantiate(self, type_args: &[TypeReference]) -> Self {
        assert_eq!(
            self.parameters.len(),
            type_args.len(),
            "expected {} type arguments, got {}",
            self.parameters.len(),
            type_args.len()
        );

        let subst = self
            .parameters
            .iter()
            .map(|p| p.name.to_owned())
            .zip(type_args.iter().cloned())
            .collect::<HashMap<_, _>>();

        Self {
            parameters: vec![],
            variants: self
                .variants
                .into_iter()
                .map(|v| v.instantiate(&subst))
                .collect(),
            ..self
        }
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        self.name = rename_type_or_module(&self.name, search_string, replacer);
        for variant in self.variants.iter_mut() {
            variant.rename_type(search_string, replacer);
        }
    }
}

impl From<Enum> for Type {
    fn from(val: Enum) -> Self {
        Type::Enum(val)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct Variant {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
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
            serde_name: String::new(),
            description: String::new(),
            fields: Vec::new(),
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

    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    pub fn discriminant(&self) -> Option<isize> {
        self.discriminant
    }

    pub fn untagged(&self) -> bool {
        self.untagged
    }

    fn rename_type(&mut self, search_string: &str, replacer: &str) {
        for field in self.fields.iter_mut() {
            field.rename_type(search_string, replacer);
        }
    }

    fn instantiate(self, subst: &Subst) -> Variant {
        Self {
            fields: self
                .fields
                .into_iter()
                .map(|f| f.instantiate(subst))
                .collect(),
            ..self
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Default)]
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

fn rename_type_or_module(name: &str, search_string: &str, replacer: &str) -> String {
    if search_string.ends_with("::") {
        // replacing module name
        if let Some(name) = name.strip_prefix(search_string) {
            format!("{replacer}{name}")
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
