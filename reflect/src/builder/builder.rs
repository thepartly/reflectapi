
pub struct Builder<S>
where
    S: Send + 'static,
{
    schema: crate::Schema,
    handlers: Vec<crate::Handler<S>>,
}

impl<S> Builder<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            schema: crate::Schema::new(),
            handlers: Vec::new(),
        }
    }

    pub fn route<F, Fut, R, I, O, E, H>(
        &mut self,
        name: &str,
        description: &str,
        readonly: bool,
        handler: F,
    ) -> &mut Self
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Into<crate::Result<O, E>> + 'static,
        I: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        H: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        O: crate::Output + serde::ser::Serialize + Send + 'static,
        E: crate::Output + serde::ser::Serialize + crate::StatusCode + Send + 'static,
    {
        let route = crate::Handler::new(name, description, readonly, handler, &mut self.schema);
        self.handlers.push(route);
        self
    }

    pub fn build(self) -> (crate::Schema, Vec<crate::Handler<S>>) {
        (self.schema, self.handlers)
    }
}
