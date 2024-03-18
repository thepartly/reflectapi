use std::{borrow::Borrow, sync::Arc};

use reflect::{Function, Schema};

use futures::future::BoxFuture;
use serde::de;

pub struct HandlerRequest {
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct HandlerResponse {
    pub body: Vec<u8>,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct HandlerError {
    pub status: http::StatusCode,
    pub body: Vec<u8>,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub struct HandlerWrapper<S>
where
    S: Send,
{
    inner: Arc<
        dyn Fn(S, HandlerRequest) -> BoxFuture<'static, Result<HandlerResponse, HandlerError>>
            + Send
            + Sync,
    >,
    marker: std::marker::PhantomData<S>,
}

impl<S> HandlerWrapper<S>
where
    S: Send + 'static,
{
    pub fn new<Fut, I, O, E>(f: fn(S, I) -> Fut) -> Self
    where
        I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
        O: reflect::Output + serde::ser::Serialize + Send + 'static,
        E: reflect::Output + serde::ser::Serialize + Send + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
    {
        // let exposed_handler = Box::new(async |state: S, input: HandlerRequest| async move {
        //     Ok(HandlerResponse {
        //         body: vec![],
        //         headers: std::collections::HashMap::new(),
        //     })
        // });

        let a = Arc::new(move |state: S, input: HandlerRequest| {
            Box::pin(handler_impl(state, input, f)) as _
        });

        Self {
            inner: a,
            marker: std::marker::PhantomData,
        }
    }

    pub fn handler(
        &self,
    ) -> Arc<
        dyn Fn(S, HandlerRequest) -> BoxFuture<'static, Result<HandlerResponse, HandlerError>>
            + Send
            + Sync,
    > {
        self.inner.clone()
    }

    pub fn call(
        &self,
        state: S,
        req: HandlerRequest,
    ) -> BoxFuture<'static, Result<HandlerResponse, HandlerError>> {
        (self.inner)(state, req)
    }
}

// impl<F, S, Fut, I, O, E> Handler<S> for F
// where
//     F: FnOnce(S, I) -> Fut + Clone + Send + 'static,
//     Fut: std::future::Future<Output = Result<O, E>> + Send,
//     I: reflect::Input + serde::de::DeserializeOwned + Send + Sized,
//     O: reflect::Output + serde::ser::Serialize + Send + Sized,
//     E: reflect::Output + serde::ser::Serialize + Send + Sized,
// {
//     type Future = std::pin::Pin<
//         Box<dyn std::future::Future<Output = Result<HandlerResponse, HandlerError>> + Send>,
//     >;

//     fn call(self, req: HandlerRequest, state: S) -> Self::Future {
//         Box::pin(async move {
//             Ok(HandlerResponse {
//                 body: vec![],
//                 headers: std::collections::HashMap::new(),
//             })
//         })
//     }
// }

pub struct SchemaWithHandlers<S>
where
    S: Send,
{
    schema: Schema,
    handlers: std::collections::HashMap<String, HandlerWrapper<S>>,
}

impl<S> SchemaWithHandlers<S>
where
    S: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            schema: Schema::new(),
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn build(self) -> (Schema, std::collections::HashMap<String, HandlerWrapper<S>>) {
        (self.schema, self.handlers)
    }

    pub fn with_function<Fut, I, O, E>(
        &mut self,
        name: &str,
        description: &str,
        handler: fn(S, I) -> Fut,
    ) -> &mut Self
    where
        // F: FnOnce(S, I) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
        I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
        O: reflect::Output + serde::ser::Serialize + Send + 'static,
        E: reflect::Output + serde::ser::Serialize + Send + 'static,
    {
        self.handlers
            .insert(name.into(), HandlerWrapper::new(handler));
        self
    }

    fn call(
        &self,
        name: &str,
        state: S,
        req: HandlerRequest,
    ) -> BoxFuture<'static, Result<HandlerResponse, HandlerError>> {
        self.handlers.get(name).unwrap().call(state, req)
    }

    // pub fn build(self) -> Schema {
    //     self.inner
    // }
}

// struct HandleImpl {

// }

// impl HandleImpl {
//     pub fn new() -> Self {
//         handler_impl(state, input, handler)
//         Self {}
//     }
// }

struct MyAppState {}

async fn example_handler(state: Arc<MyAppState>, request: u8) -> Result<u8, u8> {
    println!("hello world");
    Ok(0)
}

#[test]
fn test() {
    let mut b = SchemaWithHandlers::new();
    b.with_function("example", "example function", example_handler);
    b.call(
        "example",
        Arc::new(MyAppState {}),
        HandlerRequest {
            body: bytes::Bytes::new(),
            headers: std::collections::HashMap::new(),
        },
    );
}

async fn handler_impl<F, Fut, S, I, O, E>(
    state: S,
    input: HandlerRequest,
    handler: F,
) -> Result<HandlerResponse, HandlerError>
where
    I: reflect::Input + serde::de::DeserializeOwned,
    O: reflect::Output + serde::ser::Serialize,
    E: reflect::Output + serde::ser::Serialize,
    F: Fn(S, I) -> Fut,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
{
    let input = serde_json::from_slice::<I>(input.body.as_ref());
    let input = match input {
        Ok(r) => r,
        Err(err) => {
            return Err(HandlerError {
                status: http::StatusCode::BAD_REQUEST,
                body: format!("Failed to parse request body: {}", err).into_bytes(),
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
                        status: http::StatusCode::INTERNAL_SERVER_ERROR,
                        body: format!("Failed to serialize response body: {}", err).into_bytes(),
                        headers: std::collections::HashMap::new(),
                    });
                }
            };
            Ok(HandlerResponse {
                body: output,
                headers: std::collections::HashMap::new(),
            })
        }
        Err(err) => {
            let err = WrappedError {
                status: http::StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
                details: err,
            };
            let output = serde_json::to_vec(&err);
            let output = match output {
                Ok(r) => r,
                Err(err) => {
                    return Err(HandlerError {
                        status: http::StatusCode::INTERNAL_SERVER_ERROR,
                        body: format!("Failed to serialize response error: {}", err).into_bytes(),
                        headers: std::collections::HashMap::new(),
                    });
                }
            };
            Err(HandlerError {
                status: http::StatusCode::INTERNAL_SERVER_ERROR,
                body: output,
                headers: std::collections::HashMap::new(),
            })
        }
    };

    result
}

#[derive(serde::Serialize)]
struct WrappedError<E> {
    status: u16,
    details: E,
}
