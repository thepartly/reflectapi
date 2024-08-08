use core::fmt;
use std::sync::Arc;

use crate::{Function, Schema, Struct};

pub struct HandlerInput {
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub struct HandlerOutput {
    pub code: http::StatusCode,
    pub body: bytes::Bytes,
    pub headers: std::collections::HashMap<String, String>,
}

pub type HandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = HandlerOutput> + Send + 'static>>;

pub type HandlerCallback<S> = dyn Fn(S, HandlerInput) -> HandlerFuture + Send + Sync;

pub struct Handler<S>
where
    S: Send + 'static,
{
    pub name: String,
    pub path: String,
    pub readonly: bool,
    pub input_headers: Vec<String>,
    pub callback: Arc<HandlerCallback<S>>,
}

impl<S> Handler<S>
where
    S: Send + 'static,
{
    pub(crate) fn new<F, Fut, R, I, O, E, H>(
        name: String,
        path: String,
        description: String,
        readonly: bool,
        handler: F,
        schema: &mut Schema,
    ) -> Handler<S>
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Into<crate::Result<O, E>> + 'static,
        I: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        H: crate::Input + serde::de::DeserializeOwned + Send + 'static,
        O: crate::Output + serde::ser::Serialize + Send + 'static,
        E: crate::Output + serde::ser::Serialize + crate::StatusCode + Send + 'static,
    {
        let input_type = I::reflectapi_input_type(&mut schema.input_types);
        let input_headers = H::reflectapi_input_type(&mut schema.input_types);
        let output_type = O::reflectapi_output_type(&mut schema.output_types);
        let error_type = E::reflectapi_output_type(&mut schema.output_types);

        let mut input_headers_names = schema
            .input_types
            .get_type(input_headers.name.as_str())
            .map(|type_def| match type_def {
                crate::Type::Struct(Struct { fields, .. }) => fields
                    .iter()
                    .map(|field| field.serde_name().to_owned())
                    .collect(),
                _ => vec![],
            })
            .unwrap_or_default();

        let function_def = Function {
            name: name.clone(),
            path: path.clone(),
            description,
            input_type: if input_type.name == "reflectapi::Empty" {
                None
            } else {
                Some(input_type)
            },
            output_type: if output_type.name == "reflectapi::Empty" {
                None
            } else {
                Some(output_type)
            },
            error_type: if error_type.name == "reflectapi::Infallible" {
                None
            } else {
                Some(error_type)
            },
            input_headers: if input_headers_names.is_empty() {
                None
            } else {
                Some(input_headers)
            },
            serialization: vec![crate::SerializationMode::Json],
            readonly,
        };
        schema.functions.push(function_def);

        // inject system header requirements used by the handler wrapper
        input_headers_names.push("content-type".into());
        input_headers_names.push("traceparent".into());

        Handler {
            name,
            path,
            readonly,
            input_headers: input_headers_names.clone(),
            callback: Arc::new(move |state: S, input: HandlerInput| {
                Box::pin(Self::handler_wrap(state, input, handler)) as _
            }),
        }
    }

    async fn handler_wrap<F, Fut, R, I, H, O, E>(
        state: S,
        input: HandlerInput,
        handler: F,
    ) -> HandlerOutput
    where
        I: crate::Input + serde::de::DeserializeOwned,
        H: crate::Input + serde::de::DeserializeOwned,
        O: crate::Output + serde::ser::Serialize,
        E: crate::Output + serde::ser::Serialize + crate::StatusCode,
        F: Fn(S, I, H) -> Fut,
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Into<crate::Result<O, E>>,
    {
        let mut input_headers = input.headers;

        #[derive(Debug)]
        enum ContentType {
            Json,
            #[cfg(feature = "msgpack")]
            MessagePack,
        }

        impl fmt::Display for ContentType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    ContentType::Json => write!(f, "application/json"),
                    #[cfg(feature = "msgpack")]
                    ContentType::MessagePack => write!(f, "application/msgpack"),
                }
            }
        }

        let content_type = match input_headers.get("Content-Type").map(|s| s.as_str()) {
            #[cfg(feature = "msgpack")]
            Some("application/msgpack") => ContentType::MessagePack,
            None | Some("application/json") => ContentType::Json,
            _ => {
                return HandlerOutput {
                    code: http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    body: bytes::Bytes::from("Unsupported content type".as_bytes()),
                    headers: std::collections::HashMap::new(),
                };
            }
        };

        let input_parsed = match content_type {
            ContentType::Json => if !input.body.is_empty() {
                serde_json::from_slice::<I>(input.body.as_ref())
            } else {
                serde_json::from_value::<I>(serde_json::Value::Object(serde_json::Map::new()))
            }
            .map_err(anyhow::Error::from),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => {
                if !input.body.is_empty() {
                    rmp_serde::from_read::<_, I>(input.body.as_ref())
                } else {
                    todo!()
                    // rmp_serde::from_read::<I>(std::io::empty())
                }
                .map_err(anyhow::Error::from)
            }
        };

        let input = match input_parsed {
            Ok(r) => r,
            Err(err) => {
                return HandlerOutput {
                    code: http::StatusCode::BAD_REQUEST,
                    body: bytes::Bytes::from(
                        format!(
                            "Failed to parse request body: {}, received: {:?}",
                            err, input.body
                        )
                        .into_bytes(),
                    ),
                    headers: std::collections::HashMap::new(),
                };
            }
        };

        let mut response_headers = std::collections::HashMap::new();
        // Respond in the same format as the request
        response_headers.insert("content-type".to_string(), content_type.to_string());

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
                return HandlerOutput {
                    code: http::StatusCode::BAD_REQUEST,
                    body: bytes::Bytes::from(
                        format!("Failed to parse request headers: {}", err).into_bytes(),
                    ),
                    headers: std::collections::HashMap::new(),
                };
            }
        };

        let output = handler(state, input, input_headers).await;
        let output: crate::Result<O, E> = output.into();

        let output_serialized = match content_type {
            ContentType::Json => serde_json::to_vec(&output).map_err(anyhow::Error::from),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => rmp_serde::to_vec(&output).map_err(anyhow::Error::from),
        };

        let output_serialized = match output_serialized {
            Ok(r) => r,
            Err(err) => {
                response_headers.insert("content-type".to_string(), "text/plain".to_string());
                return HandlerOutput {
                    code: http::StatusCode::INTERNAL_SERVER_ERROR,
                    body: bytes::Bytes::from(
                        format!("Failed to serialize response body: {}", err).into_bytes(),
                    ),
                    headers: response_headers,
                };
            }
        };

        HandlerOutput {
            code: output.status_code(),
            body: bytes::Bytes::from(output_serialized),
            headers: response_headers,
        }
    }
}
