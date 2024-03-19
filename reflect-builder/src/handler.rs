use std::sync::Arc;

use reflect::{Function, Schema, Struct};

use crate::ToStatusCode;

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
    pub required_input_headers: Vec<String>,
    pub callback: Arc<HandlerCallback<S>>,
}

impl<S> Handler<S>
where
    S: Send + 'static,
{
    pub fn call(&self, state: S, req: HandlerInput) -> HandlerFuture {
        (self.callback)(state, req)
    }
}

#[derive(Clone)]
pub struct HandlerTyped<S, I, O, E, H>
where
    S: Send,
    I: reflect::Input + serde::de::DeserializeOwned + Send,
    H: reflect::Input + serde::de::DeserializeOwned + Send,
    O: reflect::Output + serde::ser::Serialize + Send,
    E: reflect::Output + serde::ser::Serialize + ToStatusCode + Send,
{
    pub callback: Arc<HandlerCallback<S>>,
    marker_i: std::marker::PhantomData<I>,
    marker_o: std::marker::PhantomData<O>,
    marker_e: std::marker::PhantomData<E>,
    marker_ih: std::marker::PhantomData<H>,
}

impl<S, I, O, E, H> HandlerTyped<S, I, O, E, H>
where
    S: Send + 'static,
    I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
    H: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
    O: reflect::Output + serde::ser::Serialize + Send + 'static,
    E: reflect::Output + serde::ser::Serialize + ToStatusCode + Send + 'static,
{
    pub fn new<F, Fut>(
        name: &str,
        description: &str,
        readonly: bool,
        f: F,
        schema: &mut Schema,
    ) -> Handler<S>
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
    {
        let input_type = I::reflect_input_type(schema);
        let output_type = O::reflect_output_type(schema);
        let error_type = E::reflect_output_type(schema);
        let input_headers = H::reflect_input_type(schema);

        let mut input_headers_names = schema
            .get_type(&input_headers.name.as_str())
            .map(|type_def| match type_def {
                reflect::Type::Struct(Struct { fields, .. }) => {
                    fields.iter().map(|field| field.name.clone()).collect()
                }
                _ => vec![],
            })
            .unwrap_or_default();

        let function_def = Function {
            name: name.into(),
            description: description.into(),
            input_type: if input_type.name == "()" {
                None
            } else {
                Some(input_type)
            },
            output_type: if output_type.name == "()" {
                None
            } else {
                Some(output_type)
            },
            error_type: if error_type.name == "reflect_builder::Infallible" {
                None
            } else {
                Some(error_type)
            },
            input_headers: if input_headers_names.is_empty() {
                None
            } else {
                Some(input_headers)
            },
            serialization: vec![reflect::SerializationMode::Json],
            readonly,
        };
        schema.functions.push(function_def);

        // inject system header rquirements used by the handler wrapper
        input_headers_names.push("content-type".into());
        input_headers_names.push("traceparent".into());

        Handler {
            name: name.into(),
            readonly,
            required_input_headers: input_headers_names.clone(),
            callback: Arc::new(move |state: S, input: HandlerInput| {
                Box::pin(Self::handler_wrap(state, input, f)) as _
            }),
        }
    }

    async fn handler_wrap<F, Fut>(
        state: S,
        input: HandlerInput,
        handler: F,
    ) -> Result<HandlerOutput, HandlerError>
    where
        I: reflect::Input + serde::de::DeserializeOwned,
        H: reflect::Input + serde::de::DeserializeOwned,
        O: reflect::Output + serde::ser::Serialize,
        E: reflect::Output + serde::ser::Serialize + ToStatusCode,
        F: Fn(S, I, H) -> Fut,
        Fut: std::future::Future<Output = Result<O, E>> + Send + 'static,
    {
        let mut input_headers = input.headers;
        let input = serde_json::from_slice::<I>(if input.body.len() == 0 {
            "{}".as_bytes()
        } else {
            input.body.as_ref()
        });
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

        let mut response_headers = std::collections::HashMap::new();
        // for now it is only json but we will add messagepack in the future depending on the request content type
        response_headers.insert("content-type".to_string(), "application/json".to_string());

        let mut headers_as_json_map = serde_json::Map::new();
        for (header_name, header_value) in input_headers.drain() {
            if header_name == "traceparent" {
                response_headers.insert("traceparent".to_string(), header_value.clone());
            } else if header_name == "content-type" {
                // TODO will be relevant when we add messagepack support
                // response_headers.insert("content-type".to_string(), header_value.clone());
            } else {
                headers_as_json_map.insert(header_name, serde_json::Value::String(header_value));
            }
        }
        let headers_as_json = serde_json::Value::Object(headers_as_json_map);

        let input_headers = serde_json::from_value::<H>(headers_as_json);
        let input_headers = match input_headers {
            Ok(r) => r,
            Err(err) => {
                return Err(HandlerError {
                    code: 400,
                    body: bytes::Bytes::from(
                        format!("Failed to parse request headers: {}", err).into_bytes(),
                    ),
                    headers: std::collections::HashMap::new(),
                });
            }
        };

        let output = handler(state, input, input_headers).await;
        let result = match output {
            Ok(output) => {
                let output = serde_json::to_vec(&output);
                let output = match output {
                    Ok(r) => r,
                    Err(err) => {
                        response_headers
                            .insert("content-type".to_string(), "text/plain".to_string());
                        return Err(HandlerError {
                            code: 500,
                            body: bytes::Bytes::from(
                                format!("Failed to serialize response body: {}", err).into_bytes(),
                            ),
                            headers: response_headers,
                        });
                    }
                };

                Ok(HandlerOutput {
                    body: bytes::Bytes::from(output),
                    headers: response_headers,
                })
            }
            Err(err) => {
                let status_code = err.to_status_code();
                let output = serde_json::to_vec(&err);
                let output = match output {
                    Ok(r) => r,
                    Err(err) => {
                        response_headers
                            .insert("content-type".to_string(), "text/plain".to_string());
                        return Err(HandlerError {
                            code: 500,
                            body: bytes::Bytes::from(
                                format!("Failed to serialize response error: {}", err).into_bytes(),
                            ),
                            headers: response_headers,
                        });
                    }
                };
                Err(HandlerError {
                    code: status_code,
                    body: bytes::Bytes::from(output),
                    headers: response_headers,
                })
            }
        };

        result
    }
}
