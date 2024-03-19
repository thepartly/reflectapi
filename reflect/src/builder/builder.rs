use crate::Schema;

use crate::{Handler, HandlerTyped, StatusCode};

pub struct Builder<S>
where
    S: Send,
{
    schema: Schema,
    handlers: Vec<Handler<S>>,
}

impl<S> Builder<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            schema: Schema::new(),
            handlers: Vec::new(),
        }
    }

    pub fn with_handler<F, Fut, R, I, O, E, H>(
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
        E: crate::Output + serde::ser::Serialize + StatusCode + Send + 'static,
    {
        let route = HandlerTyped::new(name, description, readonly, handler, &mut self.schema);
        self.handlers.push(route);
        self
    }

    pub fn build(self) -> (Schema, Vec<Handler<S>>) {
        (self.schema, self.handlers)
    }
}
