mod handler;
mod result;

use core::fmt;
use std::{borrow::Borrow, collections::BTreeSet, error::Error};

pub use handler::*;
use reflectapi_schema::Pattern;
pub use result::*;

/// [`Builder`] provides a chained API for defining the overall API specification,
/// adding individual routes (handlers), and composing multiple builders together.
///
/// # Example
///
/// ```rust
/// # use reflectapi::{Builder, RouteBuilder, Input, Output, StatusCode};
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Input, Output, Serialize, Deserialize)]
/// # struct User { id: u32, name: String }
/// # #[derive(Input, Output, Serialize, Deserialize, StatusCode)]
/// # struct ErrorResponse;
/// # impl From<ErrorResponse> for (u16, ErrorResponse) {
/// #     fn from(value: ErrorResponse) -> Self { (500, value) }
/// # }
/// #
/// # type AppState = ();
/// #
/// # async fn get_user(_state: AppState, _input: (), _headers: ()) -> Result<User, ErrorResponse> {
/// #     Ok(User { id: 1, name: "Test".to_string() })
/// # }
///
/// fn api() -> Builder<AppState> {
///     Builder::new()
///         .name("My Awesome API")
///         .description("This API manages users.")
///         .path("/api/v1")
///         .tag("Users")
///         .route(get_user, |route| {
///             route
///                 .name("GetUser")
///                 .path("/users/{id}")
///                 .description("Retrieves a single user by their ID.")
///         })
/// }
///
/// let (schema, routers) = api().build().unwrap();
/// ```
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
        I: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        H: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        O: crate::Output + serde::ser::Serialize + Send + 'static,
        E: crate::Output + serde::ser::Serialize + crate::StatusCode + Send + 'static,
    {
        let rb = builder(
            RouteBuilder::new()
                .tags(&self.default_tags)
                .path(self.path.clone()),
        );
        let route = crate::Handler::new(rb, handler, &mut self.schema);
        self.handlers.push(route);
        self
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
        self
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
    pub handlers: Vec<crate::Handler<S>>,
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
        self.tags.extend(tags.into_iter().map(|s| s.as_ref().to_string()));
        self
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
