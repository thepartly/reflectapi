use reflect_schema::EndpointSchema;

pub struct Builder<S>
where
    S: Send + 'static,
{
    schema: crate::EndpointSchema,
    handlers: Vec<crate::Handler<S>>,
}

impl<S> Builder<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            schema: crate::EndpointSchema::new(String::new(), String::new()),
            handlers: Vec::new(),
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.schema.name = name;
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
        let rb = builder(RouteBuilder::new());
        let route = crate::Handler::new(
            rb.name,
            rb.description,
            rb.readonly,
            handler,
            &mut self.schema,
        );
        self.handlers.push(route);
        self
    }

    pub fn build(
        mut self,
        renaming: Vec<(&str, &str)>,
        validation: Vec<fn(&EndpointSchema) -> Vec<crate::ValidationError>>,
    ) -> Result<(crate::EndpointSchema, Vec<crate::Handler<S>>), Vec<crate::ValidationError>> {
        for (from, to) in renaming {
            self.schema.rename_type(from, to);
        }
        let mut errors = Vec::new();
        for validator in validation {
            errors.extend(validator(&self.schema));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok((self.schema, self.handlers))
    }
}

pub struct RouteBuilder {
    name: String,
    description: String,
    readonly: bool,
}

impl RouteBuilder {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            readonly: false,
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = name;
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
