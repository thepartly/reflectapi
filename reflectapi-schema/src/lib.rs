//! The schema a reflectapi server describes itself with, and the JSON it's
//! written to.
//!
//! # Compatibility
//!
//! The schema's structs, such as [`Struct`], [`Field`] and [`Function`], are
//! `#[non_exhaustive]`, so a release can add a field to one without breaking
//! code that uses it. Build one with its constructor and set the public fields
//! you need:
//!
//! ```
//! let mut field = reflectapi_schema::Field::new("id".into(), "u64".into());
//! field.required = true;
//! let mut point = reflectapi_schema::Struct::new("geo::Point");
//! point.fields = reflectapi_schema::Fields::Named(vec![field]);
//! ```
//!
//! The enums that describe the wire, such as [`Type`], [`Fields`],
//! [`Representation`], [`OutputType`] and [`SerializationMode`], stay
//! exhaustive. A new variant of one changes what a client must send or accept,
//! so code that matches on it should have to handle it.

mod codegen;
mod deprecation;
mod internal;
mod rename;
mod subst;
pub mod transforms;
mod visit;

pub use self::codegen::*;
pub use self::deprecation::{validate_deprecation_dates, Deprecation, InvalidDeprecationDate};
pub use self::subst::{mk_subst, Instantiate, Substitute};
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

/// The schema JSON format version this crate writes, and the newest it reads.
///
/// A server writes its schema with one release of reflectapi and a client
/// project often generates code from it with another, so the JSON records the
/// format it was written in. A reader accepts any version up to its own and
/// refuses a newer one, rather than generating a client that's silently wrong.
///
/// Within a version, changes are additive: an older reader may ignore them
/// and still generate a client that makes the same requests and accepts the
/// same responses, with the same method, path, headers and body. A new
/// description, deprecation, `format` or `distinct` is additive, since it
/// only refines types and docs. A change that an older reader would get wrong
/// by ignoring it bumps the version. Examples are a new route kind, a new
/// variant of a schema enum, a field that changes the wire shape of a type,
/// as `untagged` or `open` do, and a field that changes how a route is called,
/// such as an HTTP method. Removing or renaming a field, or changing what a
/// missing field means, bumps it too.
pub const SCHEMA_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Schema {
    /// The JSON format this schema is written in: see [`SCHEMA_FORMAT_VERSION`].
    /// It's the first key written, so a reader refuses a newer format before
    /// it parses anything it might not understand. A schema without one is
    /// format 1, the format from before versions were recorded.
    ///
    /// A schema built in code starts at the current format. One that was read
    /// keeps the format it was read with, so code that gives it something
    /// newer formats introduced must raise this to match.
    #[serde(
        default = "format_version::unrecorded",
        deserialize_with = "format_version::deserialize"
    )]
    pub format_version: u32,

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

/// Just a schema's `format_version`, read without the rest of the schema.
///
/// [`Schema`]'s own check fires as it reads `format_version`, so it reports a
/// newer format only if that key comes before anything it can't parse, as it
/// does in JSON reflectapi writes. Reading the JSON into this first checks
/// the version wherever the key is, since the other keys are skipped unread:
///
/// ```
/// # let json = r#"{"name": "api", "format_version": 1}"#;
/// let _: reflectapi_schema::SchemaFormat = serde_json::from_str(json)?;
/// let schema: reflectapi_schema::Schema = serde_json::from_str(json)?;
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[non_exhaustive]
pub struct SchemaFormat {
    /// The schema's format version, checked: see [`SCHEMA_FORMAT_VERSION`].
    #[serde(
        default = "format_version::unrecorded",
        deserialize_with = "format_version::deserialize"
    )]
    pub format_version: u32,
}

mod format_version {
    use super::SCHEMA_FORMAT_VERSION;

    pub(super) fn unrecorded() -> u32 {
        1
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<u32, D::Error> {
        let version = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        if version == 0 {
            return Err(serde::de::Error::custom(
                "this schema is in format version 0, which doesn't exist: versions start at 1",
            ));
        }
        if version > SCHEMA_FORMAT_VERSION {
            return Err(serde::de::Error::custom(format!(
                "this schema is in format version {version}, but reflectapi-schema {} reads \
                 formats up to {SCHEMA_FORMAT_VERSION}: upgrade reflectapi-cli, or the \
                 reflectapi crate reading the schema, to at least the release that wrote it",
                env!("CARGO_PKG_VERSION"),
            )));
        }
        Ok(version)
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            format_version: SCHEMA_FORMAT_VERSION,
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
            format_version,
            functions,
            input_types,
            output_types,
            name: _,
            description: _,
        } = other;
        self.format_version = self.format_version.max(format_version);
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

    pub fn get_type_mut(&mut self, name: &str) -> Option<&mut Type> {
        if let Some(t) = self.input_types.get_type_mut(name) {
            return Some(t);
        }
        if let Some(t) = self.output_types.get_type_mut(name) {
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

    /// Remove fields marked as `hidden` from all struct and enum variant fields
    /// in both input and output typespaces. Intended to be called at codegen
    /// entry points so that no backend can accidentally leak hidden fields.
    pub fn strip_hidden_fields(&mut self) {
        fn strip(ts: &mut Typespace) {
            for ty in ts.types.iter_mut() {
                match ty {
                    Type::Struct(s) => s.fields.retain(|f| !f.hidden),
                    Type::Enum(e) => {
                        for v in e.variants.iter_mut() {
                            v.fields.retain(|f| !f.hidden);
                        }
                    }
                    Type::Primitive(_) => {}
                }
            }
        }
        strip(&mut self.input_types);
        strip(&mut self.output_types);
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
                    // A distinct type stays: folding it would erase it.
                    .filter(|i| i.transparent && i.fields.len() == 1 && !i.distinct)
                    .cloned()
            })
            .collect::<Vec<_>>();

        for strukt in transparent_types {
            self.input_types.remove_type(strukt.name());
            let _ = SubstVisitor::new(strukt).visit_schema_inputs(self);
        }

        let transparent_types = self
            .output_types()
            .types()
            .filter_map(|t| {
                t.as_struct()
                    // A distinct type stays: folding it would erase it.
                    .filter(|i| i.transparent && i.fields.len() == 1 && !i.distinct)
                    .cloned()
            })
            .collect::<Vec<_>>();

        for strukt in transparent_types {
            self.output_types.remove_type(strukt.name());
            let _ = SubstVisitor::new(strukt).visit_schema_outputs(self);
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

    pub fn get_type_mut(&mut self, name: &str) -> Option<&mut Type> {
        self.ensure_types_map();
        let index = {
            let b = self.types_map.borrow();
            b.get(name).copied().unwrap_or(usize::MAX)
        };
        if index == usize::MAX {
            return None;
        }
        self.types.get_mut(index)
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

        let removed = self.types.remove(index);
        // `Vec::remove` shifts every later element down by one, so all
        // map entries that pointed past `index` are now stale. Drop
        // the whole map; the next `ensure_types_map` rebuilds it.
        // (Removing the single key for `ty` and leaving the rest
        // alone — the previous behaviour — was the bug.)
        self.invalidate_types_map();
        Some(removed)
    }

    pub fn sort_types(&mut self) {
        self.types.sort_by(|a, b| a.name().cmp(b.name()));
        self.build_types_map();
    }

    pub fn has_type(&self, name: &str) -> bool {
        self.ensure_types_map();
        self.types_map.borrow().contains_key(name)
    }

    /// Removes each type that has the same name as an earlier one and is
    /// equal to it, and returns the names that still name more than one type:
    /// different types under one name, which a rename can produce. A generated
    /// client would keep only one of them.
    pub fn dedup_types(&mut self) -> Vec<String> {
        let mut kept: Vec<Type> = Vec::with_capacity(self.types.len());
        let mut kept_by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for ty in std::mem::take(&mut self.types) {
            let same_name = kept_by_name.entry(ty.name().to_string()).or_default();
            if !same_name.iter().any(|&index| kept[index] == ty) {
                same_name.push(kept.len());
                kept.push(ty);
            }
        }
        self.types = kept;
        self.invalidate_types_map();

        let mut seen = BTreeSet::new();
        let mut conflicting = BTreeSet::new();
        for ty in &self.types {
            if !seen.insert(ty.name()) {
                conflicting.insert(ty.name().to_string());
            }
        }
        conflicting.into_iter().collect()
    }

    /// The names of types both typespaces have, where `other`'s type equals
    /// none of this typespace's types of that name. [`Typespace::extend`]
    /// would keep this typespace's type for each.
    pub fn conflicting_types(&self, other: &Typespace) -> Vec<String> {
        let mut conflicting: Vec<String> = other
            .types
            .iter()
            .filter(|ty| {
                let mut same_name = self.types.iter().filter(|t| t.name() == ty.name());
                same_name.clone().next().is_some() && same_name.all(|existing| existing != *ty)
            })
            .map(|ty| ty.name().to_string())
            .collect();
        conflicting.sort();
        conflicting.dedup();
        conflicting
    }

    /// Adds `other`'s types. Where both have a type of the same name, this
    /// typespace's is kept, even if they differ: see
    /// [`Typespace::conflicting_types`].
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
#[non_exhaustive]
pub struct Function {
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
    /// Release the function was deprecated in, free-form (e.g. `2.3.0`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_in: Option<String>,
    /// Date the function was deprecated, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_since: Option<String>,
    /// Date after which the function may be removed, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sunset: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_type: Option<TypeReference>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_headers: Option<TypeReference>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_type: Option<TypeReference>,
    /// Struct of response headers that clients can read, on success and on
    /// failure. Each field is one header, named by its serde name. Declaring
    /// a header doesn't mean the server sends it: it may be added by
    /// infrastructure in front of the server (e.g. `retry-after` from a
    /// rate limiter). A header that appears more than once is read as its
    /// values joined with `, `.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub response_headers: Option<TypeReference>,

    #[serde(flatten)]
    pub output_type: OutputType,

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
            name,
            deprecation_note: Default::default(),
            deprecated_in: Default::default(),
            deprecated_since: Default::default(),
            sunset: Default::default(),
            path: Default::default(),
            description: Default::default(),
            input_type: None,
            input_headers: None,
            error_type: None,
            response_headers: None,
            output_type: OutputType::Complete { output_type: None },
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
        self.deprecation().is_some()
    }

    pub fn deprecation(&self) -> Option<Deprecation<'_>> {
        Deprecation::of(
            &self.deprecation_note,
            &self.deprecated_in,
            &self.deprecated_since,
            &self.sunset,
        )
    }

    pub fn input_type(&self) -> Option<&TypeReference> {
        self.input_type.as_ref()
    }

    pub fn input_headers(&self) -> Option<&TypeReference> {
        self.input_headers.as_ref()
    }

    pub fn response_headers(&self) -> Option<&TypeReference> {
        self.response_headers.as_ref()
    }

    pub fn output_type(&self) -> &OutputType {
        &self.output_type
    }

    pub fn serialization(&self) -> std::slice::Iter<'_, SerializationMode> {
        self.serialization.iter()
    }

    pub fn readonly(&self) -> bool {
        self.readonly
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "output_kind", rename_all = "snake_case")]
pub enum OutputType {
    Complete {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        output_type: Option<TypeReference>,
    },
    Stream {
        item_type: TypeReference,
    },
}

impl OutputType {
    pub fn type_refs(&self) -> Vec<&TypeReference> {
        match self {
            OutputType::Complete {
                output_type: Some(output_type),
            } => vec![output_type],
            OutputType::Complete { output_type: None } => vec![],
            OutputType::Stream { item_type } => vec![item_type],
        }
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
#[non_exhaustive]
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
#[non_exhaustive]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
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
        internal::replace_type_references_for_type(self, unresolved_to_resolved_map, schema, false)
    }

    /// Rebind derived input fields and resolve serde's missing-option behavior.
    #[doc(hidden)]
    pub fn __internal_rebind_input_generic_parameters(
        &mut self,
        unresolved_to_resolved_map: &std::collections::HashMap<TypeReference, TypeReference>,
        schema: &Typespace,
    ) {
        internal::replace_type_references_for_type(self, unresolved_to_resolved_map, schema, true)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
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

    /// What a value is on the wire, beyond its JSON type: a format from the
    /// OpenAPI format registry, such as `uuid`, `date-time`, `date`,
    /// `date-time-local` (no offset) or `uri`, or another name a type
    /// chooses. Clients keep the meaning it gives (OpenAPI's `format`, a
    /// named type) without each knowing the Rust type.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<String>,

    #[serde(
        skip_serializing_if = "LanguageSpecificTypeCodegenConfig::is_serialization_default",
        default
    )]
    pub codegen_config: LanguageSpecificTypeCodegenConfig,
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
            format: None,
            codegen_config: Default::default(),
        }
    }

    /// Sets what a value is on the wire, as a JSON Schema `format` such as
    /// `uuid` or `date-time`; see [`Primitive::format`].
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// What a value is on the wire beyond its JSON type, if the type says.
    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[non_exhaustive]
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

    pub fields: Fields,

    /// If serde transparent attribute is set on a struct
    #[serde(skip_serializing_if = "is_false", default)]
    pub transparent: bool,

    /// `#[reflectapi(distinct)]`: a newtype, or a one-field transparent
    /// struct, that clients keep apart from the type it wraps, as Rust does.
    /// On the wire it's still the wrapped value.
    #[serde(skip_serializing_if = "is_false", default)]
    pub distinct: bool,

    #[serde(
        skip_serializing_if = "LanguageSpecificTypeCodegenConfig::is_serialization_default",
        default
    )]
    pub codegen_config: LanguageSpecificTypeCodegenConfig,
}

impl Struct {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Struct {
            name,
            serde_name: Default::default(),
            description: Default::default(),
            parameters: Default::default(),
            fields: Default::default(),
            transparent: Default::default(),
            distinct: Default::default(),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq, Default)]
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

    /// Keep only fields for which `predicate` returns `true`.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Field) -> bool,
    {
        match self {
            Fields::Named(fields) | Fields::Unnamed(fields) => fields.retain(|f| predicate(f)),
            Fields::None => {}
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq)]
#[non_exhaustive]
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

    /// Deprecation note. If none, field is not deprecated.
    /// If present as empty string, field is deprecated without a note.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecation_note: Option<String>,
    /// Release the field was deprecated in, free-form (e.g. `2.3.0`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_in: Option<String>,
    /// Date the field was deprecated, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_since: Option<String>,
    /// Date after which the field may be removed, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sunset: Option<String>,

    /// Type of a field
    #[serde(rename = "type")]
    pub type_ref: TypeReference,
    /// Whether the field's key is required in this type's wire direction.
    /// For inputs, a missing required key is rejected; for outputs, a required
    /// key is always serialized. Value nullability is represented by the type.
    /// Derived input option fields accept a missing key unless a custom
    /// deserializer prevents serde's missing-option behavior. User transforms
    /// and explicitly constructed schemas may override this contract.
    ///
    /// required and not nullable:
    /// - field always present and not null / none
    ///
    /// required and nullable:
    /// - Rust: `Option<T>`, do not skip serializing if None
    /// - TypeScript: T | null, do not skip serializing if null
    ///
    /// not required and not nullable:
    /// - Rust: `Option<T>`, skip serializing if None
    /// - TypeScript: T | undefined, skip serializing if undefined
    ///
    /// not required and nullable:
    ///   serializers and deserializers are required to differentiate between
    ///   missing fields and null / none fields
    /// - Rust: `reflectapi::Option<T>` is enum with Undefined, None and Some variants
    /// - TypeScript: T | null | undefined
    ///
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub required: bool,
    /// If serde flatten attribute is set on a field
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub flattened: bool,

    /// If true, the field is excluded from generated clients and documentation
    /// but is still functional at runtime (e.g. for header extraction).
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub hidden: bool,

    /// If true, the field has a custom serde serializer:
    /// `#[serde(serialize_with = ...)]` or `#[serde(with = ...)]`.
    /// Serialized wire shape cannot be inferred from the field's type alone.
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub serialize_with: bool,

    /// If true, the field has a custom serde deserializer:
    /// `#[serde(deserialize_with = ...)]` or `#[serde(with = ...)]`.
    /// Deserialization behavior — such as whether a missing key is
    /// accepted — cannot be inferred from the field's type alone.
    /// Default is false
    #[serde(skip_serializing_if = "is_false", default)]
    pub deserialize_with: bool,

    #[serde(skip, default)]
    pub transform_callback: String,
    #[serde(skip, default)]
    pub transform_callback_fn: Option<fn(&mut Field, &Typespace) -> ()>,
}

impl PartialEq for Field {
    fn eq(
        &self,
        Self {
            name,
            serde_name,
            description,
            deprecation_note,
            deprecated_in,
            deprecated_since,
            sunset,
            type_ref,
            required,
            flattened,
            hidden,
            serialize_with,
            deserialize_with,
            transform_callback,
            transform_callback_fn: _,
        }: &Self,
    ) -> bool {
        self.name == *name
            && self.serde_name == *serde_name
            && self.description == *description
            && self.deprecation_note == *deprecation_note
            && self.deprecated_in == *deprecated_in
            && self.deprecated_since == *deprecated_since
            && self.sunset == *sunset
            && self.type_ref == *type_ref
            && self.required == *required
            && self.flattened == *flattened
            && self.hidden == *hidden
            && self.serialize_with == *serialize_with
            && self.deserialize_with == *deserialize_with
            && self.transform_callback == *transform_callback
    }
}

impl std::hash::Hash for Field {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.serde_name.hash(state);
        self.description.hash(state);
        self.deprecation_note.hash(state);
        self.deprecated_in.hash(state);
        self.deprecated_since.hash(state);
        self.sunset.hash(state);
        self.type_ref.hash(state);
        self.required.hash(state);
        self.flattened.hash(state);
        self.hidden.hash(state);
        self.serialize_with.hash(state);
        self.deserialize_with.hash(state);
        self.transform_callback.hash(state);
    }
}

impl Field {
    pub fn new(name: String, type_ref: TypeReference) -> Self {
        Field {
            name,
            type_ref,
            serde_name: Default::default(),
            description: Default::default(),
            deprecation_note: Default::default(),
            deprecated_in: Default::default(),
            deprecated_since: Default::default(),
            sunset: Default::default(),
            required: Default::default(),
            flattened: Default::default(),
            hidden: Default::default(),
            serialize_with: Default::default(),
            deserialize_with: Default::default(),
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
        self.deprecation().is_some()
    }

    pub fn deprecation(&self) -> Option<Deprecation<'_>> {
        Deprecation::of(
            &self.deprecation_note,
            &self.deprecated_in,
            &self.deprecated_since,
            &self.sunset,
        )
    }

    pub fn hidden(&self) -> bool {
        self.hidden
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

    pub fn transform_callback_fn(&self) -> Option<fn(&mut Field, &Typespace)> {
        self.transform_callback_fn
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[non_exhaustive]
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

    /// Generated response clients retain unfamiliar enum tags and their raw JSON.
    /// This policy is never reflected into request types.
    #[serde(skip_serializing_if = "is_false", default)]
    pub open: bool,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub variants: Vec<Variant>,

    #[serde(
        skip_serializing_if = "LanguageSpecificTypeCodegenConfig::is_serialization_default",
        default
    )]
    pub codegen_config: LanguageSpecificTypeCodegenConfig,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            serde_name: Default::default(),
            description: Default::default(),
            parameters: Default::default(),
            representation: Default::default(),
            open: false,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct Variant {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub serde_name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
    /// Deprecation note. If none, variant is not deprecated.
    /// If present as empty string, variant is deprecated without a note.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecation_note: Option<String>,
    /// Release the variant was deprecated in, free-form (e.g. `2.3.0`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_in: Option<String>,
    /// Date the variant was deprecated, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_since: Option<String>,
    /// Date after which the variant may be removed, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sunset: Option<String>,

    pub fields: Fields,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discriminant: Option<isize>,

    /// If serde `untagged` attribute is set on a variant
    #[serde(skip_serializing_if = "is_false", default)]
    pub untagged: bool,

    /// Serde's unit catch-all for an internally or adjacently tagged enum.
    #[serde(skip_serializing_if = "is_false", default)]
    pub other: bool,
}

impl Variant {
    pub fn new(name: String) -> Self {
        Variant {
            name,
            serde_name: String::new(),
            description: String::new(),
            deprecation_note: None,
            deprecated_in: None,
            deprecated_since: None,
            sunset: None,
            fields: Fields::None,
            discriminant: None,
            untagged: false,
            other: false,
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

    pub fn deprecated(&self) -> bool {
        self.deprecation().is_some()
    }

    pub fn deprecation(&self) -> Option<Deprecation<'_>> {
        Deprecation::of(
            &self.deprecation_note,
            &self.deprecated_in,
            &self.deprecated_since,
            &self.sunset,
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn primitive(name: &str) -> Type {
        Type::Primitive(Primitive {
            name: name.to_string(),
            description: String::new(),
            parameters: vec![],
            fallback: None,
            format: None,
            codegen_config: LanguageSpecificTypeCodegenConfig::default(),
        })
    }

    /// Regression: `remove_type` used to drop only the removed key
    /// from `types_map` while `Vec::remove` shifted every later
    /// element's index down by one. The next call would either
    /// panic on an out-of-bounds slot or silently return the wrong
    /// type. The fix is to invalidate the whole map after removal
    /// so the next access rebuilds it.
    #[test]
    fn remove_type_keeps_map_consistent_across_multiple_removals() {
        let mut ts = Typespace::default();
        ts.insert_type(primitive("a"));
        ts.insert_type(primitive("b"));
        ts.insert_type(primitive("c"));
        ts.insert_type(primitive("d"));

        // Remove two — the second one used to land on a stale index.
        let _ = ts.remove_type("b");
        let _ = ts.remove_type("c");

        assert!(ts.has_type("a"), "untouched type 'a' should still resolve");
        assert!(ts.has_type("d"), "untouched type 'd' should still resolve");
        assert!(!ts.has_type("b"));
        assert!(!ts.has_type("c"));

        // Each surviving lookup should return the right Type, not
        // some other slot that the stale index pointed at.
        assert_eq!(ts.get_type("a").map(|t| t.name()), Some("a"));
        assert_eq!(ts.get_type("d").map(|t| t.name()), Some("d"));
    }

    #[test]
    fn remove_type_returns_the_value_we_asked_for() {
        let mut ts = Typespace::default();
        ts.insert_type(primitive("first"));
        ts.insert_type(primitive("second"));
        ts.insert_type(primitive("third"));

        let removed = ts.remove_type("second").expect("should find 'second'");
        assert_eq!(removed.name(), "second");

        // The other two must still be there at the right names.
        assert_eq!(ts.get_type("first").map(|t| t.name()), Some("first"));
        assert_eq!(ts.get_type("third").map(|t| t.name()), Some("third"));
    }
}
