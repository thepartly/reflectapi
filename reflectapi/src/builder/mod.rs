mod event;
mod handler;
mod mode;
mod result;

use core::fmt;
use std::{
    borrow::Borrow,
    collections::{BTreeSet, HashMap},
    error::Error,
    marker::PhantomData,
};

pub use event::{Event, EventId, InvalidEventId, StreamItem};
use futures_util::Stream;
pub use handler::*;
pub use mode::{BuildMode, DispatchOnly, WithSchema};
use reflectapi_schema::Pattern;
#[allow(deprecated)]
pub use result::StatusCode;
pub use result::{IntoResult, ResponseError};
use serde::{de::DeserializeOwned, ser::Serialize};

use crate::{Input, Output};

/// `Output::reflectapi_output_type` of some type: registers it in the output
/// typespace and returns a reference to it.
type OutputTypeFn = fn(&mut crate::Typespace) -> crate::TypeReference;

/// [`Builder`] provides a chained API for defining the overall API specification,
/// adding individual routes (handlers), and composing multiple builders together.
///
/// `M` is the [`BuildMode`]: [`WithSchema`] (the default) builds the schema
/// and the routes; [`DispatchOnly`] builds the routes alone, for serving.
pub struct Builder<S, M = WithSchema>
where
    S: Send + 'static,
    M: BuildMode,
{
    mode: PhantomData<M>,
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

impl<S, M> fmt::Debug for Builder<S, M>
where
    S: Send + 'static,
    M: BuildMode,
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

impl<S, M> Default for Builder<S, M>
where
    S: Send + 'static,
    M: BuildMode,
{
    /// Creates a new, empty [`Builder`] in mode `M`. Use it to start an API
    /// definition that is generic over the [`BuildMode`].
    fn default() -> Self {
        Self {
            mode: PhantomData,
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
}

impl<S> Builder<S, WithSchema>
where
    S: Send + 'static,
{
    /// Creates a new, empty [`Builder`] that builds the schema and the routes.
    /// For a definition generic over the [`BuildMode`], start from
    /// [`Builder::default()`].
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, M> Builder<S, M>
where
    S: Send + 'static,
    M: BuildMode,
{
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
        self.default_response_headers = M::response_headers_type::<H>();
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
        E: ResponseError + Send + 'static,
    {
        let rb = builder(self.route_defaults());
        if rb.sse_keep_alive.is_some() {
            self.errors.push(BuildError::Other(
                format!(
                    "route `{}`: `sse_keep_alive` applies only to stream routes",
                    rb.name
                )
                .into(),
            ));
            return self;
        }
        match M::register_function::<I, H, O, E>(&rb, &mut self.schema, false) {
            Ok(input_headers) => {
                self.handlers
                    .push(crate::Handler::new(rb, handler, input_headers))
            }
            Err(err) => self.errors.push(err),
        }
        self
    }

    /// Adds a stream route to the API.
    ///
    /// This method takes a stream handler function and a closure that configures the
    /// route's metadata (like its name, path, and description) using a [`RouteBuilder`].
    /// The handler is async, as a route's is: it may wait before it returns its
    /// error or its stream. The stream yields its items as they are, or each in an
    /// [`Event`] to send it with an event id.
    pub fn stream_route<F, Fut, St, I, O, E1, H>(
        mut self,
        handler: F,
        builder: fn(RouteBuilder) -> RouteBuilder,
    ) -> Self
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = Result<St, E1>> + Send + 'static,
        St: Stream<Item = O> + Send + 'static,
        I: Input + DeserializeOwned + Send + 'static,
        H: Input + DeserializeOwned + Send + 'static,
        O: StreamItem + Send + 'static,
        E1: ResponseError + Send + 'static,
    {
        let rb = builder(self.route_defaults());
        match M::register_function::<I, H, O::Data, E1>(&rb, &mut self.schema, true) {
            Ok(input_headers) => {
                self.handlers
                    .push(crate::Handler::new_stream(rb, handler, input_headers))
            }
            Err(err) => self.errors.push(err),
        }
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
    pub fn extend(mut self, mut other: Builder<S, M>) -> Self {
        if M::WITH_SCHEMA {
            // Merging keeps one type per name, so `other`'s own collisions
            // must be found before they're merged away. This builder's stay
            // for `build` to find.
            self.errors
                .extend(dedup_types(&mut other.schema, RENAME_COLLISION));
            for (side, ours, theirs) in [
                (
                    "request",
                    &self.schema.input_types,
                    &other.schema.input_types,
                ),
                (
                    "response",
                    &self.schema.output_types,
                    &other.schema.output_types,
                ),
            ] {
                for name in ours.conflicting_types(theirs) {
                    self.errors.push(BuildError::Other(
                        format!(
                            "the builders being merged each have a {side} type named `{name}`, \
                             and they differ, so generated clients would keep only one: \
                             rename one of them"
                        )
                        .into(),
                    ));
                }
            }
        }
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
            mode: self.mode,
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
    pub fn nest(self, other: Builder<S, M>) -> Self {
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
        if !M::WITH_SCHEMA {
            return self;
        }
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

    /// Inlines all `#[serde(transparent)]` types throughout the schema.
    /// This simplifies the schema by replacing wrapper types with their inner types.
    pub fn fold_transparent_types(mut self) -> Self {
        if M::WITH_SCHEMA {
            // Folding rewrites every reference to a transparent type's name,
            // so a collision under that name must be found first.
            self.errors
                .extend(dedup_types(&mut self.schema, RENAME_COLLISION));
            self.schema.fold_transparent_types();
        }
        self
    }

    /// The schema as built so far and the routes, unless errors were recorded.
    fn into_parts(
        self,
    ) -> std::result::Result<(crate::Schema, Vec<Router<S>>), crate::BuildErrors> {
        if !self.errors.is_empty() {
            // A problem found at more than one step, such as a name
            // collision, is reported once.
            let mut seen = std::collections::HashSet::new();
            let errors = self
                .errors
                .into_iter()
                .filter(|error| seen.insert(error.to_string()))
                .collect();
            return Err(crate::BuildErrors(errors));
        }

        let mut routers = vec![Router {
            name: self.schema.name.clone(),
            handlers: self.handlers,
        }];

        for (name, handlers) in self.merged_handlers {
            routers.push(Router { name, handlers });
        }

        Ok((self.schema, routers))
    }
}

impl<S> Builder<S, WithSchema>
where
    S: Send + 'static,
{
    /// Consumes the builder and attempts to build the final API [`crate::Schema`] and [`Vec<Router>`].
    ///
    /// This method performs final validation and consolidation. It returns an error
    /// if any validation checks fail or if any other build errors were recorded.
    pub fn build(
        mut self,
    ) -> std::result::Result<(crate::Schema, Vec<Router<S>>), crate::BuildErrors> {
        self.errors
            .extend(dedup_types(&mut self.schema, RENAME_COLLISION));
        drop_unused_type_parameters(&mut self.schema);
        self.schema.input_types.sort_types();
        self.schema.output_types.sort_types();

        for validator in self.validators.iter() {
            for err in validator(&self.schema) {
                self.errors.push(BuildError::Validation(err));
            }
        }
        self.errors.extend(validate_response_headers(&self.schema));
        self.errors.extend(validate_deprecation_dates(&self.schema));
        self.errors.extend(validate_flattened_options(&self.schema));
        self.errors.extend(validate_untagged_units(&self.schema));

        let (mut schema, routers) = self.into_parts()?;
        schema.consolidate_types();
        let mut errors = dedup_types(&mut schema, CONSOLIDATION_COLLISION);
        // On the consolidated names, which the clients are generated from.
        errors.extend(validate_module_names(&schema));
        if !errors.is_empty() {
            return Err(crate::BuildErrors(errors));
        }

        Ok((schema, routers))
    }
}

impl<S> Builder<S, DispatchOnly>
where
    S: Send + 'static,
{
    /// Consumes the builder and builds the routes, without a schema.
    ///
    /// Schema validation (custom validators, response headers, redundant
    /// renames) is skipped: it runs where the schema is built, with
    /// [`WithSchema`]. Errors that affect serving, such as an invalid request
    /// header name, are still reported.
    pub fn build(self) -> std::result::Result<Vec<Router<S>>, crate::BuildErrors> {
        self.into_parts().map(|(_, routers)| routers)
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
    deprecated_in: Option<String>,
    deprecated_since: Option<String>,
    sunset: Option<String>,
    response_headers: Option<OutputTypeFn>,
    sse_keep_alive: Option<std::time::Duration>,
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

    /// Sets the path for the route, in place of the parent [`Builder`]'s base path.
    ///
    /// The route is served at this path followed by `/{name}`, and paths of
    /// builders this one is nested into are still prepended. The path will be
    /// normalized to ensure it starts with a `/` and does not end with one.
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

    /// Marks this route as deprecated in `release`, a free-form string such
    /// as a version (`2.3.0`). Rendered as `x-deprecated-in` in OpenAPI.
    pub fn deprecated_in(mut self, release: impl Into<String>) -> Self {
        self.deprecated_in = Some(release.into());
        self
    }

    /// Marks this route as deprecated since `date`, a `YYYY-MM-DD` string.
    /// [`Builder::build`] fails if the date is malformed.
    pub fn deprecated_since(mut self, date: impl Into<String>) -> Self {
        self.deprecated_since = Some(date.into());
        self
    }

    /// Marks this route as deprecated and sets the date after which it may be
    /// removed, a `YYYY-MM-DD` string. Rendered as `x-sunset` in OpenAPI.
    /// [`Builder::build`] fails if the date is malformed or before
    /// [`Self::deprecated_since`].
    pub fn sunset(mut self, date: impl Into<String>) -> Self {
        self.sunset = Some(date.into());
        self
    }

    /// For a stream route, writes an SSE comment whenever the stream has sent
    /// nothing for `interval`, overriding the server's default
    /// (`reflectapi::axum::RouterSettings`, or the host's for a
    /// [`crate::dispatch::Dispatcher`]). `Duration::ZERO` turns it off for
    /// this route. It is a serving setting: the schema and generated clients
    /// don't change. Setting it on a route that isn't a stream route is a
    /// build error.
    pub fn sse_keep_alive(mut self, interval: std::time::Duration) -> Self {
        self.sse_keep_alive = Some(interval);
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

/// Why two different types can share a name, for [`dedup_types`]' error.
const RENAME_COLLISION: &str =
    "a rename probably maps different types onto this name, so rename one of them";
const CONSOLIDATION_COLLISION: &str =
    "a request and a response type that shared a name were renamed apart into `input` and \
     `output` namespaces, and a type already had the new name, so rename one of them";

/// Merges copies of one type under one name, and reports different types
/// under one name: every generated client would keep only one of them.
fn dedup_types(schema: &mut crate::Schema, cause: &str) -> Vec<BuildError> {
    let mut errors = Vec::new();
    for (side, types) in [
        ("request", &mut schema.input_types),
        ("response", &mut schema.output_types),
    ] {
        for name in types.dedup_types() {
            errors.push(BuildError::Other(
                format!(
                    "`{name}` names more than one {side} type, so generated clients would keep \
                     only one: {cause}"
                )
                .into(),
            ));
        }
    }
    errors
}

/// Drops the type parameters a struct's or enum's schema doesn't use, and the
/// matching arguments wherever it is used. A parameter that reflection erases,
/// such as `C` in a field `Option<Encoded<C>>`, which reflects as
/// `Option<String>`, is then not in the schema, and neither is a type only
/// reachable through it: clients never see a type that exists only on the
/// server. Built-in types (primitives) keep their parameters.
fn drop_unused_type_parameters(schema: &mut crate::Schema) {
    for side in [Side::Input, Side::Output] {
        let mut roots: Vec<&mut crate::TypeReference> = Vec::new();
        let typespace = match side {
            Side::Input => {
                for f in schema.functions.iter_mut() {
                    roots.extend(f.input_type.as_mut());
                    roots.extend(f.input_headers.as_mut());
                }
                &mut schema.input_types
            }
            Side::Output => {
                for f in schema.functions.iter_mut() {
                    match &mut f.output_type {
                        crate::OutputType::Complete { output_type } => {
                            roots.extend(output_type.as_mut())
                        }
                        crate::OutputType::Stream { item_type } => roots.push(item_type),
                    }
                    roots.extend(f.error_type.as_mut());
                    roots.extend(f.response_headers.as_mut());
                }
                &mut schema.output_types
            }
        };
        drop_unused_type_parameters_in(typespace, roots);
    }
}

#[derive(Clone, Copy)]
enum Side {
    Input,
    Output,
}

fn drop_unused_type_parameters_in(
    typespace: &mut crate::Typespace,
    mut roots: Vec<&mut crate::TypeReference>,
) {
    let reachable_before = reachable_types(typespace, roots.iter().map(|r| &**r));
    let mut dropped_any = false;
    loop {
        // Which parameter positions each generic struct or enum doesn't use.
        let mut dropped: HashMap<String, Vec<usize>> = HashMap::new();
        for ty in typespace.types() {
            let parameters: Vec<&str> = match ty {
                crate::Type::Struct(s) => s.parameters().map(|p| p.name.as_str()).collect(),
                crate::Type::Enum(e) => e.parameters().map(|p| p.name.as_str()).collect(),
                crate::Type::Primitive(_) => continue,
            };
            if parameters.is_empty() {
                continue;
            }
            let mut used = BTreeSet::new();
            for_each_field_type(ty, |type_ref| collect_names(type_ref, &mut used));
            let unused: Vec<usize> = parameters
                .iter()
                .enumerate()
                .filter(|(_, name)| !used.contains(**name))
                .map(|(index, _)| index)
                .collect();
            if !unused.is_empty() {
                dropped.insert(ty.name().to_owned(), unused);
            }
        }
        if dropped.is_empty() {
            break;
        }
        dropped_any = true;

        let names: Vec<String> = typespace.types().map(|ty| ty.name().to_owned()).collect();
        for name in &names {
            let Some(ty) = typespace.get_type_mut(name) else {
                continue;
            };
            if let Some(unused) = dropped.get(name) {
                let parameters = match ty {
                    crate::Type::Struct(s) => &mut s.parameters,
                    crate::Type::Enum(e) => &mut e.parameters,
                    crate::Type::Primitive(_) => unreachable!("primitives keep their parameters"),
                };
                remove_positions(parameters, unused);
            }
            for_each_field_type_mut(ty, |type_ref| drop_arguments(type_ref, &dropped));
        }
        for root in roots.iter_mut() {
            drop_arguments(root, &dropped);
        }
    }
    if !dropped_any {
        return;
    }
    // Types only reachable through a dropped argument go; types nothing used
    // before are left as they were.
    let reachable_after = reachable_types(typespace, roots.iter().map(|r| &**r));
    for name in reachable_before.difference(&reachable_after) {
        typespace.remove_type(name);
    }
}

fn remove_positions<T>(items: &mut Vec<T>, positions: &[usize]) {
    let mut index = 0;
    items.retain(|_| {
        let keep = !positions.contains(&index);
        index += 1;
        keep
    });
}

fn drop_arguments(type_ref: &mut crate::TypeReference, dropped: &HashMap<String, Vec<usize>>) {
    if let Some(unused) = dropped.get(&type_ref.name) {
        remove_positions(&mut type_ref.arguments, unused);
    }
    for argument in &mut type_ref.arguments {
        drop_arguments(argument, dropped);
    }
}

fn collect_names<'a>(type_ref: &'a crate::TypeReference, names: &mut BTreeSet<&'a str>) {
    names.insert(&type_ref.name);
    for argument in &type_ref.arguments {
        collect_names(argument, names);
    }
}

/// The type of every field of a struct or an enum's variants, and a
/// primitive's fallback.
fn for_each_field_type<'a>(ty: &'a crate::Type, mut f: impl FnMut(&'a crate::TypeReference)) {
    match ty {
        crate::Type::Struct(s) => s.fields().for_each(|field| f(&field.type_ref)),
        crate::Type::Enum(e) => e
            .variants()
            .flat_map(|v| v.fields())
            .for_each(|field| f(&field.type_ref)),
        crate::Type::Primitive(p) => p.fallback().into_iter().for_each(f),
    }
}

fn for_each_field_type_mut(ty: &mut crate::Type, mut f: impl FnMut(&mut crate::TypeReference)) {
    match ty {
        crate::Type::Struct(s) => s.fields.iter_mut().for_each(|field| f(&mut field.type_ref)),
        crate::Type::Enum(e) => e
            .variants
            .iter_mut()
            .flat_map(|v| v.fields.iter_mut())
            .for_each(|field| f(&mut field.type_ref)),
        crate::Type::Primitive(p) => p.fallback.iter_mut().for_each(f),
    }
}

/// The names of the types reachable from `roots`, through fields, arguments
/// and fallbacks.
fn reachable_types<'a>(
    typespace: &crate::Typespace,
    roots: impl Iterator<Item = &'a crate::TypeReference>,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending: Vec<crate::TypeReference> = roots.cloned().collect();
    while let Some(type_ref) = pending.pop() {
        pending.extend(type_ref.arguments.iter().cloned());
        if !seen.insert(type_ref.name.clone()) {
            continue;
        }
        if let Some(ty) = typespace.get_type(&type_ref.name) {
            for_each_field_type(ty, |field_type| pending.push(field_type.clone()));
        }
    }
    seen
}

/// Field and variant dates are checked when their type is derived; route
/// dates come from the builder and are only checked here.
fn validate_deprecation_dates(schema: &crate::Schema) -> Vec<BuildError> {
    schema
        .functions
        .iter()
        .filter_map(|f| {
            crate::validate_deprecation_dates(f.deprecated_since.as_deref(), f.sunset.as_deref())
                .err()
                .map(|err| {
                    BuildError::Other(format!("route `{}` deprecation: {err}", f.name).into())
                })
        })
        .collect()
}

/// Rejects `#[serde(flatten)]` on a `reflectapi::Option` field. The type means a
/// key that can be absent, null or set, and a flattened field has no key: its
/// fields are either in the parent or not, which is what std's `Option` models.
/// serde cannot deserialize the flattened `reflectapi::Option` anyway.
fn validate_flattened_options(schema: &crate::Schema) -> Vec<BuildError> {
    let mut checked = BTreeSet::new();
    let mut errors = Vec::new();
    for ty in schema
        .input_types
        .types()
        .chain(schema.output_types.types())
    {
        if !checked.insert(ty.name().to_owned()) {
            continue;
        }
        let fields: Vec<(String, &crate::Field)> = match ty {
            crate::Type::Struct(s) => s.fields().map(|f| (f.name().to_owned(), f)).collect(),
            crate::Type::Enum(e) => e
                .variants()
                .flat_map(|v| {
                    v.fields()
                        .map(move |f| (format!("{}.{}", v.name(), f.name()), f))
                })
                .collect(),
            crate::Type::Primitive(_) => continue,
        };
        for (path, field) in fields {
            if field.flattened && field.type_ref.name == "reflectapi::Option" {
                errors.push(BuildError::Other(
                    format!(
                        "`{}.{path}` flattens a `reflectapi::Option`, which has no key to be absent or null; use `Option<T>`",
                        ty.name()
                    )
                    .into(),
                ));
            }
        }
    }
    errors
}

/// Rejects untagged unit variants that lose data. Such a variant is written as
/// `null`, so two in one enum can't be told apart (`null` reads back as the
/// first), and one inside an `Option` reads back as `None`. One `null` variant
/// elsewhere is a value of its own, and allowed.
fn validate_untagged_units(schema: &crate::Schema) -> Vec<BuildError> {
    let mut errors = BTreeSet::new();
    for typespace in [&schema.input_types, &schema.output_types] {
        for ty in typespace.types() {
            if let crate::Type::Enum(e) = ty {
                if let [first, rest @ ..] = null_variants(e).as_slice() {
                    if !rest.is_empty() {
                        let all = std::iter::once(first)
                            .chain(rest)
                            .map(|v| format!("`{v}`"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        errors.insert(format!(
                            "`{}`: variants {all} are untagged units, so each is written as `null`, and `null` reads back as `{first}`",
                            e.name()
                        ));
                    }
                }
            }
            let fields: Vec<(String, &crate::Field)> = match ty {
                crate::Type::Struct(s) => {
                    s.fields().map(|f| (f.serde_name().to_owned(), f)).collect()
                }
                crate::Type::Enum(e) => e
                    .variants()
                    .flat_map(|v| {
                        v.fields()
                            .map(move |f| (format!("{}.{}", v.name(), f.serde_name()), f))
                    })
                    .collect(),
                crate::Type::Primitive(_) => continue,
            };
            for (path, field) in fields {
                let location = format!("`{}.{path}`", ty.name());
                check_optional_null(&field.type_ref, typespace, &location, &mut errors);
            }
        }
    }
    for f in &schema.functions {
        let inputs = [
            ("input", f.input_type()),
            ("input headers", f.input_headers()),
        ];
        let outputs = [
            (
                "output",
                match f.output_type() {
                    crate::OutputType::Complete { output_type } => output_type.as_ref(),
                    crate::OutputType::Stream { item_type } => Some(item_type),
                },
            ),
            ("error", f.error_type.as_ref()),
            ("response headers", f.response_headers()),
        ];
        for (typespace, refs) in [
            (&schema.input_types, &inputs[..]),
            (&schema.output_types, &outputs[..]),
        ] {
            for (what, type_ref) in refs {
                if let Some(type_ref) = type_ref {
                    let location = format!("route `{}` {what}", f.name);
                    check_optional_null(type_ref, typespace, &location, &mut errors);
                }
            }
        }
    }
    errors
        .into_iter()
        .map(|error| BuildError::Other(error.into()))
        .collect()
}

/// The variants of `e` written as `null`: its unit variants if it is
/// untagged, else its unit variants marked `#[serde(untagged)]`. The options'
/// own `None` are what they mean, and not counted.
fn null_variants(e: &crate::Enum) -> Vec<&str> {
    if matches!(e.name(), "std::option::Option" | "reflectapi::Option") {
        return Vec::new();
    }
    let untagged = matches!(e.representation, crate::Representation::None);
    e.variants()
        .filter(|v| matches!(v.fields, crate::Fields::None) && (untagged || v.untagged))
        .map(|v| v.name())
        .collect()
}

/// Reports an `Option` or `reflectapi::Option`, anywhere in `type_ref`,
/// holding an enum with a variant written as `null`.
fn check_optional_null(
    type_ref: &crate::TypeReference,
    typespace: &crate::Typespace,
    location: &str,
    errors: &mut BTreeSet<String>,
) {
    if matches!(
        type_ref.name.as_str(),
        "std::option::Option" | "reflectapi::Option"
    ) {
        if let Some(inner) = type_ref.arguments.first() {
            if let Some(crate::Type::Enum(e)) = typespace.get_type(&inner.name) {
                if let Some(variant) = null_variants(e).first() {
                    errors.insert(format!(
                        "{location} is `{}<{}>`, and `{}::{variant}` is an untagged unit, written as `null`, which reads back as `None`",
                        type_ref.name,
                        e.name(),
                        e.name(),
                    ));
                }
            }
        }
    }
    for argument in &type_ref.arguments {
        check_optional_null(argument, typespace, location, errors);
    }
}

/// Rejects a reference the TypeScript and Python clients would resolve to the
/// wrong module. They look a name up from the innermost scope outward, so in
/// a type inside `c`, a reference to `a::b::X` breaks when a module `c::a`
/// exists: `a::…` is read as `c::a::…`. Nesting a module under a top-level
/// one's name is fine while nothing in its scope refers to the top-level one.
fn validate_module_names(schema: &crate::Schema) -> Vec<BuildError> {
    let types = || {
        schema
            .input_types
            .types()
            .chain(schema.output_types.types())
    };
    // Every module a type is in: `a` and `a::b` for `a::b::X`.
    let modules: BTreeSet<String> = types()
        .flat_map(|ty| {
            let segments: Vec<&str> = ty.name().split("::").collect();
            (1..segments.len())
                .map(|end| segments[..end].join("::"))
                .collect::<Vec<_>>()
        })
        .collect();

    let mut errors = BTreeSet::new();
    for ty in types() {
        let segments: Vec<&str> = ty.name().split("::").collect();
        let module = &segments[..segments.len() - 1];
        let mut references = Vec::new();
        match ty {
            crate::Type::Struct(s) => {
                references.extend(s.fields().map(|f| &f.type_ref));
            }
            crate::Type::Enum(e) => {
                references.extend(e.variants().flat_map(|v| v.fields()).map(|f| &f.type_ref));
            }
            crate::Type::Primitive(_) => continue,
        }
        while let Some(type_ref) = references.pop() {
            references.extend(&type_ref.arguments);
            let Some(root) = type_ref.name.split("::").next() else {
                continue;
            };
            if !type_ref.name.contains("::") {
                // A type parameter, or a type outside any module.
                continue;
            }
            // The innermost enclosing module with a child named `root`.
            if let Some(shadowing) = (1..=module.len())
                .rev()
                .map(|end| format!("{}::{root}", module[..end].join("::")))
                .find(|candidate| modules.contains(candidate))
            {
                errors.insert(format!(
                    "`{}` refers to `{}`, but inside `{}` the TypeScript and Python clients read `{root}::…` as `{shadowing}::…`; rename one of the modules with `rename_types`",
                    ty.name(),
                    type_ref.name,
                    module.join("::"),
                ));
            }
        }
    }
    errors
        .into_iter()
        .map(|error| BuildError::Other(error.into()))
        .collect()
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
#[non_exhaustive]
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
