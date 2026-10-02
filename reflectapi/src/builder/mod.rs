mod handler;
mod result;

use core::fmt;
use std::{borrow::Borrow, collections::BTreeSet, error::Error};

use futures_util::Stream;
pub use handler::*;
use reflectapi_schema::Pattern;
pub use result::{IntoResult, StatusCode};
use serde::{de::DeserializeOwned, ser::Serialize};

use crate::{Input, Output};

/// `Output::reflectapi_output_type` of some type: registers it in the output
/// typespace and returns a reference to it.
type OutputTypeFn = fn(&mut crate::Typespace) -> crate::TypeReference;

/// [`Builder`] provides a chained API for defining the overall API specification,
/// adding individual routes (handlers), and composing multiple builders together.
pub struct Builder<S>
where
    S: Send + 'static,
{
    schema: crate::Schema,
    path: String,
    handlers: Vec<crate::Handler<S>>,
    merged_handlers: Vec<(String, Vec<crate::Handler<S>>)>,
    validators: Vec<fn(&crate::Schema) -> Vec<crate::ValidationError>>,
    allow_redundant_renames: bool,
    errors: Vec<BuildError>,
    default_tags: BTreeSet<String>,
    default_response_headers: Option<OutputTypeFn>,
}

impl<S> fmt::Debug for Builder<S>
where
    S: Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("schema", &self.schema)
            .field("path", &self.path)
            .field("handlers", &self.handlers)
            .field("merged_handlers", &self.merged_handlers)
            .field("default_tags", &self.default_tags)
            .field(
                "default_response_headers",
                &self.default_response_headers.is_some(),
            )
            .finish()
    }
}

impl<S> Default for Builder<S>
where
    S: Send + 'static,
{
    /// Creates a new, empty [`Builder`]. Equivalent to [`Builder::new()`].
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Builder<S>
where
    S: Send + 'static,
{
    /// Creates a new, empty [`Builder`].
    pub fn new() -> Self {
        Self {
            schema: Default::default(),
            path: Default::default(),
            handlers: Default::default(),
            merged_handlers: Default::default(),
            validators: Default::default(),
            errors: Default::default(),
            allow_redundant_renames: Default::default(),
            default_tags: Default::default(),
            default_response_headers: Default::default(),
        }
    }

    /// Sets the top-level name for the API schema.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.schema.name = name.into();
        self
    }

    /// Sets a base path to be prepended to all routes defined in this builder.
    ///
    /// The path will be normalized to ensure it starts with a `/` (if not empty)
    /// and does not end with one.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        self.path = path;
        if self.path.ends_with('/') {
            self.path.pop();
        }
        if !self.path.starts_with('/') && !self.path.is_empty() {
            self.path.insert(0, '/');
        }
        self
    }

    /// Configures whether to record an error if a `rename_types` operation
    /// matches no types.
    ///
    /// By default, this is `false`, and a redundant rename will result in a `BuildError`.
    pub fn allow_redundant_renames(mut self, allow: bool) -> Self {
        self.allow_redundant_renames = allow;
        self
    }

    /// Sets the top-level description for the API schema.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.schema.description = description.into();
        self
    }

    /// Adds a default tag to be applied to all routes in this builder.
    pub fn tag<T: AsRef<str>>(mut self, tag: T) -> Self {
        self.default_tags.insert(tag.as_ref().into());
        self
    }

    /// Adds multiple default tags to be applied to all routes in this builder.
    pub fn tags<T: AsRef<str>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        self.default_tags
            .extend(tags.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Removes a default tag from this builder.
    pub fn untag<Q>(mut self, tag: &Q) -> Self
    where
        String: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.default_tags.remove(tag);
        self
    }

    /// Declares the response headers that clients can read, for every route
    /// of this builder that doesn't declare its own: routes added before or
    /// after this call, and routes of builders merged in with
    /// [`Builder::extend`] or [`Builder::nest`] that have no default of
    /// their own.
    ///
    /// `H` is a struct with one `Option<T>` field per header, where `T` is a
    /// string on the wire (`String`, a unit-variant enum, a newtype over one,
    /// `uuid::Uuid`, `chrono::DateTime`, ...): the header's value is
    /// deserialized into it, as for request headers. The field's serde name is
    /// the header name and must be lowercase. See
    /// [`crate::Function::response_headers`]. Override per route with
    /// [`RouteBuilder::response_headers`]; routes that already declare
    /// response headers, including from an earlier call to this method, keep
    /// them. As with routes, call [`Builder::rename_types`] afterwards so the
    /// rename applies to the headers type.
    pub fn response_headers<H: Output>(mut self) -> Self {
        self.default_response_headers = Some(H::reflectapi_output_type);
        self.apply_default_response_headers();
        self
    }

    /// Adds a route to the API.
    ///
    /// This method takes a handler function and a closure that configures the
    /// route's metadata (like its name, path, and description) using a [`RouteBuilder`].
    pub fn route<F, Fut, R, I, O, E, H>(
        mut self,
        handler: F,
        builder: fn(RouteBuilder) -> RouteBuilder,
    ) -> Self
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: IntoResult<O, E> + 'static,
        I: Input + DeserializeOwned + Send + 'static,
        H: Input + DeserializeOwned + Send + 'static,
        O: Output + Serialize + Send + 'static,
        E: Output + Serialize + StatusCode + Send + 'static,
    {
        let rb = builder(self.route_defaults());
        let route = crate::Handler::new(rb, handler, &mut self.schema);
        self.handlers.push(route);
        self
    }

    /// Adds a stream route to the API.
    ///
    /// This method takes a stream handler function and a closure that configures the
    /// route's metadata (like its name, path, and description) using a [`RouteBuilder`].
    pub fn stream_route<F, St, I, O, E1, H>(
        mut self,
        handler: F,
        builder: fn(RouteBuilder) -> RouteBuilder,
    ) -> Self
    where
        F: Fn(S, I, H) -> Result<St, E1> + Send + Sync + Copy + 'static,
        St: Stream<Item = O> + Send + 'static,
        I: Input + DeserializeOwned + Send + 'static,
        H: Input + DeserializeOwned + Send + 'static,
        O: Output + Serialize + Send + 'static,
        E1: Output + Serialize + StatusCode + Send + 'static,
    {
        let rb = builder(self.route_defaults());
        let route = crate::Handler::new_stream(rb, handler, &mut self.schema);
        self.handlers.push(route);
        self
    }

    fn route_defaults(&self) -> RouteBuilder {
        RouteBuilder {
            response_headers: self.default_response_headers,
            ..RouteBuilder::new()
        }
        .tags(&self.default_tags)
        .path(self.path.clone())
    }

    /// Gives every function already in the schema without its own response
    /// headers this builder's default, if it has one. Routes added later get
    /// it in `route_defaults`.
    fn apply_default_response_headers(&mut self) {
        let Some(reflect_output_type) = self.default_response_headers else {
            return;
        };
        let schema = &mut self.schema;
        let mut default = None;
        for function in schema
            .functions
            .iter_mut()
            .filter(|f| f.response_headers.is_none())
        {
            let type_ref = default
                .get_or_insert_with(|| reflect_output_type(&mut schema.output_types))
                .clone();
            function.response_headers = Some(type_ref);
        }
    }

    /// Merges another [`Builder`] into this one.
    ///
    /// The schema definitions and handlers from `other` are merged.
    /// The handlers from `other` are grouped into a separate [`Router`] identified
    /// by `other`'s name. This is useful for combining independent API modules.
    /// Note: This does not prepend any paths. Use [`Builder::nest`] for hierarchical routing.
    pub fn extend(mut self, other: Builder<S>) -> Self {
        let other_name = other.schema.name.clone();
        self.merged_handlers.push((other_name, other.handlers));
        self.schema.extend(other.schema);
        self.apply_default_response_headers();
        self.errors.extend(other.errors);
        self.validators.extend(other.validators);

        // Don't merge `allow_redundant_renames`, `default_tags` or
        // `default_response_headers`, as these are configuration options that
        // should be set per-builder. `other`'s default response headers are
        // already on its routes.

        // Explicitly reconstruct Self to ensure new fields are handled appropriately.
        Self {
            schema: self.schema,
            path: self.path,
            handlers: self.handlers,
            merged_handlers: self.merged_handlers,
            validators: self.validators,
            allow_redundant_renames: self.allow_redundant_renames,
            errors: self.errors,
            default_tags: self.default_tags,
            default_response_headers: self.default_response_headers,
        }
    }

    /// Nests another [`Builder`] under this one's base path.
    ///
    /// This is the primary method for composing modular APIs. It merges the schema
    /// and handlers from `other`, and prepends this builder's `path` to all of
    /// `other`'s routes.
    pub fn nest(self, other: Builder<S>) -> Self {
        let other = other.prepend_path(self.path.as_str());
        self.extend(other)
    }

    /// Internal helper to prepend a path to all handlers and schema paths.
    fn prepend_path(mut self, path: &str) -> Self {
        if path.is_empty() {
            return self;
        }
        self.schema.prepend_path(path);
        for h in self.handlers.iter_mut() {
            h.path = format!("{}{}", path, h.path);
        }
        self
    }

    /// Renames types in the schema that match a glob pattern.
    ///
    /// This is a powerful tool for cleaning up type names, especially for removing
    /// verbose module paths.
    ///
    /// # Example
    ///
    /// `builder.glob_rename_types("my_crate::models::*", "")`
    #[cfg(feature = "glob")]
    pub fn glob_rename_types<G: AsRef<str>, R: AsRef<str>>(mut self, glob: G, replacer: R) -> Self {
        match glob.as_ref().parse::<reflectapi_schema::Glob>() {
            Ok(pattern) => self.rename_types(&pattern, replacer.as_ref()),
            Err(err) => {
                self.errors.push(BuildError::Other(
                    format!("invalid glob pattern: {err}").into(),
                ));
                self
            }
        }
    }

    /// Renames types in the schema that match a given pattern.
    pub fn rename_types(mut self, pattern: impl Pattern + fmt::Display, to: &str) -> Self {
        if self.schema.rename_types(pattern, to) == 0 && !self.allow_redundant_renames {
            self.errors.push(BuildError::RedundantRename {
                pattern: pattern.to_string(),
            });
        }

        self
    }

    /// Adds a custom validation function to be run against the schema during the build process.
    ///
    /// This allows for enforcing project-specific rules, such as naming conventions or
    /// ensuring all routes have descriptions.
    pub fn validate(
        mut self,
        validation: fn(&crate::Schema) -> Vec<crate::ValidationError>,
    ) -> Self {
        self.validators.push(validation);
        self
    }

    /// Inlines all types marked as `#[reflectapi(transparent)]` throughout the schema.
    /// This simplifies the schema by replacing wrapper types with their inner types.
    pub fn fold_transparent_types(mut self) -> Self {
        self.schema.fold_transparent_types();
        self
    }

    /// Consumes the builder and attempts to build the final API [`crate::Schema`] and [`Vec<Router>`].
    ///
    /// This method performs final validation and consolidation. It returns an error
    /// if any validation checks fail or if any other build errors were recorded.
    pub fn build(
        mut self,
    ) -> std::result::Result<(crate::Schema, Vec<Router<S>>), crate::BuildErrors> {
        self.schema.input_types.sort_types();
        self.schema.output_types.sort_types();

        for validator in self.validators.iter() {
            for err in validator(&self.schema) {
                self.errors.push(BuildError::Validation(err));
            }
        }
        self.errors.extend(validate_response_headers(&self.schema));

        if !self.errors.is_empty() {
            return Err(crate::BuildErrors(self.errors));
        }

        let mut routers = vec![Router {
            name: self.schema.name.clone(),
            handlers: self.handlers,
        }];

        for (name, handlers) in self.merged_handlers {
            let router = Router {
                name: name.clone(),
                handlers,
            };
            routers.push(router);
        }

        self.schema.consolidate_types();

        Ok((self.schema, routers))
    }
}

/// A collection of named handlers, produced by the [`Builder`].
pub struct Router<S>
where
    S: Send + 'static,
{
    /// The name of this router, derived from the [`Builder`]'s name.
    pub name: String,
    /// The list of handlers belonging to this router.
    pub(crate) handlers: Vec<crate::Handler<S>>,
}

/// A fluent builder for configuring a single route's metadata.
///
/// An instance of [`RouteBuilder`] is passed to the closure in [`Builder::route`].
#[derive(Default)]
pub struct RouteBuilder {
    name: String,
    path: String,
    description: String,
    readonly: bool,
    tags: BTreeSet<String>,
    deprecation_note: Option<String>,
    response_headers: Option<OutputTypeFn>,
}

impl RouteBuilder {
    /// Creates a new, empty [`RouteBuilder`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the name for the route, often used as an "operation ID" in API specifications.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the path for the route.
    ///
    /// If the parent [`Builder`] has a base path, this path will be appended to it.
    /// The path will be normalized to ensure it starts with a `/` and does not end with one.
    pub fn path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = path.as_ref().into();
        if self.path.ends_with('/') {
            self.path.pop();
        }
        if !self.path.starts_with('/') && !self.path.is_empty() {
            self.path.insert(0, '/');
        }
        self
    }

    /// Sets the description for the route, used for documentation.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Marks this route as deprecated, with an optional explanatory note.
    /// If the provided string is empty, the route is marked as deprecated without a note.
    pub fn deprecation_note(mut self, deprecated: impl Into<String>) -> Self {
        self.deprecation_note = Some(deprecated.into());
        self
    }

    /// Marks this route as "read-only".
    /// This is a hint to code generators that the route likely corresponds to
    /// an HTTP GET request and does not modify server state.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Adds a tag to the route, used for grouping related operations in documentation.
    pub fn tag<T: AsRef<str>>(mut self, tag: T) -> Self {
        self.tags.insert(tag.as_ref().into());
        self
    }

    /// Removes a tag from the route.
    pub fn untag<Q>(mut self, tag: &Q) -> Self
    where
        String: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.tags.remove(tag);
        self
    }

    /// Adds multiple tags to the route.
    pub fn tags<T: AsRef<str>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        self.tags
            .extend(tags.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Declares the response headers that clients can read for this route,
    /// replacing the builder default. See [`Builder::response_headers`].
    pub fn response_headers<H: Output>(mut self) -> Self {
        self.response_headers = Some(H::reflectapi_output_type);
        self
    }
}

/// Checks each distinct `response_headers` type: a non-generic struct of
/// named `Option<T>` fields, where `T` is a string on the wire (as request
/// headers are parsed: the header's string value is deserialized into the
/// field), with lowercase header names.
fn validate_response_headers(schema: &crate::Schema) -> Vec<BuildError> {
    let mut checked = BTreeSet::new();
    let mut errors = Vec::new();
    for type_ref in schema.functions.iter().filter_map(|f| f.response_headers()) {
        if !checked.insert(type_ref.name.clone()) {
            continue;
        }
        let invalid = |reason: String| {
            BuildError::Other(format!("response headers type `{}`: {reason}", type_ref.name).into())
        };
        let fields = match schema.get_type(&type_ref.name) {
            Some(crate::Type::Struct(s)) if type_ref.arguments.is_empty() && !s.is_tuple() => {
                s.fields()
            }
            _ => {
                errors.push(invalid(
                    "must be a non-generic struct with named fields".into(),
                ));
                continue;
            }
        };
        let mut seen_names = BTreeSet::new();
        for field in fields {
            let header_name = field.serde_name();
            if !seen_names.insert(header_name) {
                errors.push(invalid(format!(
                    "`{header_name}` is declared by more than one field"
                )));
            }
            let type_problem = if field.flattened {
                Some("is flattened; declare each header as its own field")
            } else if field.type_ref.name == "reflectapi::Option" {
                Some("must be `Option<T>`, not `reflectapi::Option`: a header is absent or present, never null")
            } else if field.type_ref.name != "std::option::Option" {
                Some("must be an `Option`: any response header may be absent")
            } else if !matches!(
                field.type_ref.arguments.as_slice(),
                [value_type] if is_string_on_the_wire(schema, value_type, 0)
            ) {
                Some("must be `Option<T>` where `T` is a string on the wire, e.g. `String`, a unit-variant enum, a newtype over one, `uuid::Uuid` or `chrono::DateTime`")
            } else {
                None
            };
            if let Some(problem) = type_problem {
                errors.push(invalid(format!("header `{header_name}` {problem}")));
            }
            if http::HeaderName::from_bytes(header_name.as_bytes()).is_err()
                || header_name != header_name.to_ascii_lowercase()
            {
                errors.push(invalid(format!(
                    "`{header_name}` is not a valid lowercase header name"
                )));
            }
        }
    }
    errors
}

/// Whether values of `type_ref` serialize as a JSON string, so a header's
/// string value deserializes into it.
fn is_string_on_the_wire(
    schema: &crate::Schema,
    type_ref: &crate::TypeReference,
    depth: usize,
) -> bool {
    // String primitives that have no `fallback` to `String` in their schema.
    const STRING_PRIMITIVES: [&str; 3] = ["std::string::String", "char", "uuid::Uuid"];
    if STRING_PRIMITIVES.contains(&type_ref.name.as_str()) {
        return true;
    }
    if depth > 8 {
        return false;
    }
    match schema.get_type(&type_ref.name) {
        Some(crate::Type::Primitive(p)) => p.fallback.as_ref().is_some_and(|fallback| {
            // A fallback can be one of the primitive's type parameters, as
            // `Box<T>` falls back to `T`: resolve it to the argument given.
            let fallback = p
                .parameters()
                .filter(|parameter| !parameter.name.starts_with('\''))
                .position(|parameter| parameter.name == fallback.name)
                .and_then(|index| type_ref.arguments.get(index))
                .unwrap_or(fallback);
            is_string_on_the_wire(schema, fallback, depth + 1)
        }),
        Some(crate::Type::Enum(e)) => {
            e.representation.is_external()
                && e.variants()
                    .all(|v| matches!(v.fields, crate::Fields::None) && !v.untagged())
        }
        Some(crate::Type::Struct(s))
            if s.fields.len() == 1 && (s.transparent() || s.is_tuple()) =>
        {
            s.fields()
                .next()
                .is_some_and(|field| is_string_on_the_wire(schema, &field.type_ref, depth + 1))
        }
        _ => false,
    }
}

/// An error that can occur during the [`Builder::build`] process.
#[derive(Debug)]
pub enum BuildError {
    /// An error from a custom validation function.
    Validation(crate::ValidationError),
    /// A generic error.
    Other(Box<dyn Error + Send + Sync>),
    /// A [`Builder::rename_types`] operation was configured but did not match any types.
    RedundantRename { pattern: String },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(err) => write!(f, "{err}"),
            Self::RedundantRename { pattern } => {
                write!(f, "pattern `{pattern}` did not match any types")
            }
            Self::Other(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// A collection of errors that occurred during the build process.
///
/// See [`BuildError`] for more details.
#[derive(Debug)]
pub struct BuildErrors(pub Vec<BuildError>);

impl IntoIterator for BuildErrors {
    type Item = BuildError;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for BuildErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for err in &self.0 {
            writeln!(f, "{err}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BuildErrors {}
