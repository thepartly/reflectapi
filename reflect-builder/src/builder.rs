use std::{fmt::Display, sync::Arc};

use reflect::{Function, Schema};

pub struct HandlerInput {
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct HandlerOutput {
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct HandlerError {
    pub code: u16,
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub type HandlerFuture = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<HandlerOutput, HandlerError>> + Send + 'static>,
>;

pub type HandlerCallback<S> = dyn Fn(S, HandlerInput) -> HandlerFuture + Send + Sync;

#[derive(Clone)]
pub struct Handler<S>
where
    S: Send,
{
    pub name: String,
    pub readonly: bool,
    pub callback: Arc<HandlerCallback<S>>,
}

impl<S> Handler<S>
where
    S: Send + 'static,
{
    pub fn new<F, Fut, I, O, E>(name: String, readonly: bool, f: F) -> Self
    where
        I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
        O: reflect::Output + serde::ser::Serialize + Send + 'static,
        E: reflect::Output + ToStatusCode + Display + serde::ser::Serialize + Send + 'static,
        F: Fn(S, I) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
    {
        Self {
            name,
            readonly,
            callback: Arc::new(move |state: S, input: HandlerInput| {
                Box::pin(handler_impl(state, input, f)) as _
            }),
        }
    }

    pub fn call(&self, state: S, req: HandlerInput) -> HandlerFuture {
        (self.callback)(state, req)
    }
}

pub trait ToStatusCode {
    fn to_status_code(&self) -> u16;
}

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

    pub fn with_function<Fut, I, O, E, F>(
        &mut self,
        name: &str,
        description: &str,
        handler: F,
        readonly: bool,
    ) -> &mut Self
    where
        F: Fn(S, I) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
        I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
        O: reflect::Output + serde::ser::Serialize + Send + 'static,
        E: reflect::Output + ToStatusCode + Display + serde::ser::Serialize + Send + 'static,
    {
        let input_type = Some(I::reflect_input_type(&mut self.schema));
        let output_type = Some(O::reflect_output_type(&mut self.schema));
        let error_type = Some(E::reflect_output_type(&mut self.schema));

        self.handlers
            .push(Handler::new(name.into(), readonly, handler));
        self.schema.functions.push(Function {
            name: name.into(),
            description: description.into(),
            input_type,
            output_type,
            error_type,
            input_headers: None,
            output_headers: None,
            error_headers: None,
            serialization: vec![reflect::SerializationMode::Json],
            readonly,
        });
        self
    }

    pub fn build(self) -> (Schema, Vec<Handler<S>>) {
        (self.schema, self.handlers)
    }
}

#[derive(serde::Serialize)]
struct WrappedError<E> {
    code: u16,
    message: String,
    details: E,
}

async fn handler_impl<F, Fut, S, I, O, E>(
    state: S,
    input: HandlerInput,
    handler: F,
) -> Result<HandlerOutput, HandlerError>
where
    I: reflect::Input + serde::de::DeserializeOwned,
    O: reflect::Output + serde::ser::Serialize,
    E: reflect::Output + ToStatusCode + Display + serde::ser::Serialize,
    F: Fn(S, I) -> Fut,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
{
    let input = serde_json::from_slice::<I>(input.body.as_ref());
    let input = match input {
        Ok(r) => r,
        Err(err) => {
            return Err(HandlerError {
                code: 400,
                body: bytes::Bytes::from(
                    format!("Failed to parse request body: {}", err).into_bytes(),
                ),
                headers: std::collections::HashMap::new(),
            });
        }
    };

    let output = handler(state, input).await;
    let result = match output {
        Ok(output) => {
            let output = serde_json::to_vec(&output);
            let output = match output {
                Ok(r) => r,
                Err(err) => {
                    return Err(HandlerError {
                        code: 500,
                        body: bytes::Bytes::from(
                            format!("Failed to serialize response body: {}", err).into_bytes(),
                        ),
                        headers: std::collections::HashMap::new(),
                    });
                }
            };
            Ok(HandlerOutput {
                body: bytes::Bytes::from(output),
                headers: std::collections::HashMap::new(),
            })
        }
        Err(err) => {
            let status_code = err.to_status_code();
            let err = WrappedError {
                code: status_code,
                message: err.to_string(),
                details: err,
            };
            let output = serde_json::to_vec(&err);
            let output = match output {
                Ok(r) => r,
                Err(err) => {
                    return Err(HandlerError {
                        code: 500,
                        body: bytes::Bytes::from(
                            format!("Failed to serialize response error: {}", err).into_bytes(),
                        ),
                        headers: std::collections::HashMap::new(),
                    });
                }
            };
            Err(HandlerError {
                code: status_code,
                body: bytes::Bytes::from(output),
                headers: std::collections::HashMap::new(),
            })
        }
    };

    result
}
