mod handler;
mod result;

pub use handler::*;
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
            schema: crate::Schema::new(),
            path: String::from(""),
            handlers: Vec::new(),
            merged_handlers: Vec::new(),
            validators: Vec::new(),
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
        R: Into<crate::Result<O, E>> + 'static,
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

    pub fn rename_type(mut self, from: &str, to: &str) -> Self {
        self.schema.rename_type(from, to);
        self
    }

    pub fn fold_transparent_types(mut self) -> Self {
        self.schema.fold_transparent_types();
        self
    }

    pub fn consolidate_types(mut self) -> Self {
        self.schema.consolidate_types();
        self
    }

    pub fn validate(
        mut self,
        validation: fn(&crate::Schema) -> Vec<crate::ValidationError>,
    ) -> Self {
        self.validators.push(validation);
        self
    }

    pub fn build(
        mut self,
    ) -> anyhow::Result<(crate::Schema, Vec<Router<S>>), Vec<crate::ValidationError>> {
        self.schema.input_types.sort_types();
        self.schema.output_types.sort_types();

        let mut errors = Vec::new();
        for validator in self.validators.iter() {
            errors.extend(validator(&self.schema));
        }
        if !errors.is_empty() {
            return Err(errors);
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
