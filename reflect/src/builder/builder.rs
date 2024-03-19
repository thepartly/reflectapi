use crate::Schema;

use crate::{Handler, HandlerTyped, ToStatusCode};

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

    pub fn with_handler<F, Fut, I, O, E, H>(
        &mut self,
        name: &str,
        description: &str,
        readonly: bool,
        handler: F,
    ) -> &mut Self
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
        I: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        H: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        O: crate::Output + serde::ser::Serialize + Send + 'static,
        E: crate::Output + serde::ser::Serialize + ToStatusCode + Send + 'static,
    {
        let route = HandlerTyped::new(name, description, readonly, handler, &mut self.schema);
        self.handlers.push(route);
        self
    }

    pub fn with_handler_infallible<F, Fut, I, O, H>(
        &mut self,
        name: &str,
        description: &str,
        readonly: bool,
        handler: F,
    ) -> &mut Self
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
        I: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        H: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        O: crate::Output + serde::ser::Serialize + Send + 'static,
    {
        self.with_handler(name, description, readonly, move |s, i, ih| async move {
            let r = handler(s, i, ih).await;
            Ok::<O, crate::Infallible>(r)
        })
    }

    pub fn build(self) -> (Schema, Vec<Handler<S>>) {
        (self.schema, self.handlers)
    }
}
