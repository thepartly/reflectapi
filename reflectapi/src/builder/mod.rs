mod handler;
mod result;

use core::fmt;
use std::error::Error;

pub use handler::*;
use reflectapi_schema::Pattern;
pub use result::*;

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
            .finish()
    }
}

impl<S> Default for Builder<S>
where
    S: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Builder<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            schema: Default::default(),
            path: Default::default(),
            handlers: Default::default(),
            merged_handlers: Default::default(),
            validators: Default::default(),
            errors: Default::default(),
            allow_redundant_renames: Default::default(),
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.schema.name = name;
        self
    }

    pub fn path(mut self, path: String) -> Self {
        self.path = path;
        if self.path.ends_with('/') {
            self.path.pop();
        }
        if !self.path.starts_with('/') && !self.path.is_empty() {
            self.path.insert(0, '/');
        }
        self
    }

    pub fn allow_redundant_renames(mut self, allow: bool) -> Self {
        self.allow_redundant_renames = allow;
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.schema.description = description;
        self
    }

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
        let rb = builder(RouteBuilder::new().path(self.path.clone()));
        let route = crate::Handler::new(
            rb.name,
            rb.path,
            rb.description,
            rb.readonly,
            handler,
            &mut self.schema,
        );
        self.handlers.push(route);
        self
    }

    pub fn extend(mut self, other: Builder<S>) -> Self {
        let other_name = other.schema.name.clone();
        self.merged_handlers.push((other_name, other.handlers));
        self.schema.extend(other.schema);
        self
    }

    pub fn nest(self, other: Builder<S>) -> Self {
        let other = other.prepend_path(self.path.as_str());
        self.extend(other)
    }

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

    #[cfg(feature = "glob")]
    pub fn glob_rename_types(mut self, glob: &str, replacer: &str) -> Self {
        match glob.parse::<reflectapi_schema::Glob>() {
            Ok(pattern) => self.rename_types(&pattern, replacer),
            Err(err) => {
                self.errors.push(BuildError::Other(
                    format!("invalid glob pattern: {err}").into(),
                ));
                self
            }
        }
    }

    pub fn rename_types(mut self, pattern: impl Pattern + fmt::Display, to: &str) -> Self {
        if self.schema.rename_types(pattern, to) == 0 && !self.allow_redundant_renames {
            self.errors.push(BuildError::RedundantRename {
                pattern: pattern.to_string(),
            });
        }

        self
    }

    pub fn validate(
        mut self,
        validation: fn(&crate::Schema) -> Vec<crate::ValidationError>,
    ) -> Self {
        self.validators.push(validation);
        self
    }

    pub fn fold_transparent_types(mut self) -> Self {
        self.schema.fold_transparent_types();
        self
    }

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

pub struct Router<S>
where
    S: Send + 'static,
{
    pub name: String,
    pub handlers: Vec<crate::Handler<S>>,
}

pub struct RouteBuilder {
    name: String,
    path: String,
    description: String,
    readonly: bool,
}

impl Default for RouteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteBuilder {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            path: String::from(""),
            description: String::new(),
            readonly: false,
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn path(mut self, path: String) -> Self {
        self.path = path;
        if self.path.ends_with('/') {
            self.path.pop();
        }
        if !self.path.starts_with('/') && !self.path.is_empty() {
            self.path.insert(0, '/');
        }
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
}

#[derive(Debug)]
pub enum BuildError {
    Validation(crate::ValidationError),
    Other(Box<dyn Error + Send + Sync>),
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
