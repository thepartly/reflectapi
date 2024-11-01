use core::fmt;
use std::sync::Arc;

use crate::{Function, Schema, Struct};

pub struct HandlerInput {
    pub body: bytes::Bytes,
    pub headers: http::HeaderMap,
}

pub struct HandlerOutput {
    pub code: http::StatusCode,
    pub body: bytes::Bytes,
    pub headers: http::HeaderMap,
}

// Not public API, currently exposed for tests only
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    Json,
    #[cfg(feature = "msgpack")]
    MessagePack,
}

impl TryFrom<&http::HeaderMap> for ContentType {
    type Error = anyhow::Error;

    fn try_from(value: &http::HeaderMap) -> Result<Self, Self::Error> {
        match value.get("content-type") {
            Some(v) => {
                let mime = v.to_str()?.parse::<mime::Mime>()?;
                if let Some(charset) = mime.get_param(mime::CHARSET) {
                    if charset != mime::UTF_8 {
                        return Err(anyhow::anyhow!("unsupported charset: {charset}"));
                    }
                }

                if mime.type_() != mime::APPLICATION {
                    return Err(anyhow::anyhow!("unsupported media type: {mime}"));
                }

                match mime.subtype() {
                    mime::JSON => Ok(ContentType::Json),
                    #[cfg(feature = "msgpack")]
                    mime::MSGPACK => Ok(ContentType::MessagePack),
                    _ => anyhow::bail!("unsupported content-type: {mime}"),
                }
            }
            None => Ok(ContentType::Json),
        }
    }
}

impl From<ContentType> for http::HeaderValue {
    fn from(t: ContentType) -> http::HeaderValue {
        http::HeaderValue::from_static(match t {
            ContentType::Json => "application/json",
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => "application/msgpack",
        })
    }
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

impl<S> fmt::Debug for Handler<S>
where
    S: Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("readonly", &self.readonly)
            .field("input_headers", &self.input_headers)
            .finish()
    }
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
        R: crate::IntoResult<O, E> + 'static,
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
        R: crate::IntoResult<O, E>,
    {
        let mut input_headers = input.headers;

        let content_type = match ContentType::try_from(&input_headers) {
            Ok(t) => t,
            Err(_) => {
                return HandlerOutput {
                    code: http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    body: "Unsupported content type".into(),
                    headers: Default::default(),
                };
            }
        };

        let input_parsed = match content_type {
            ContentType::Json => if !input.body.is_empty() {
                serde_json::from_slice::<I>(input.body.as_ref())
            } else {
                serde_json::from_value::<I>(serde_json::Value::Object(serde_json::Map::new()))
            }
            .map_err(|err| err.to_string()),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => if !input.body.is_empty() {
                rmp_serde::from_read::<_, I>(input.body.as_ref())
            } else {
                rmp_serde::from_slice::<I>(&[0x80])
            }
            .map_err(|err| err.to_string()),
        };

        let input = match input_parsed {
            Ok(r) => r,
            Err(err) => {
                return HandlerOutput {
                    code: http::StatusCode::BAD_REQUEST,
                    body: bytes::Bytes::from(
                        format!(
                            "Failed to parse request body: {err}, received: {:?}",
                            input.body
                        )
                        .into_bytes(),
                    ),
                    headers: Default::default(),
                };
            }
        };

        let mut response_headers = http::HeaderMap::new();
        // Respond in the same format as the request
        response_headers.insert("content-type", content_type.into());

        let mut headers_as_json_map = serde_json::Map::new();
        let mut current_header_name = None;
        for (header_name, header_value) in input_headers.drain() {
            let header_name = match header_name {
                Some(header_name) => {
                    current_header_name = Some(header_name.clone());
                    header_name
                }
                None => current_header_name.as_ref().cloned().unwrap(),
            };

            if header_name == "traceparent" {
                response_headers.insert("traceparent", header_value.clone());
            } else {
                headers_as_json_map.insert(
                    header_name.to_string(),
                    serde_json::Value::String(header_value.to_str().unwrap_or_default().to_owned()),
                );
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
                    headers: Default::default(),
                };
            }
        };

        let output = handler(state, input, input_headers).await;
        let output = UntaggedResult::from(output.into_result());

        let output_serialized = match content_type {
            ContentType::Json => serde_json::to_vec_pretty(&output).map_err(|err| err.to_string()),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => {
                rmp_serde::to_vec_named(&output).map_err(|err| err.to_string())
            }
        };

        let output_serialized = match output_serialized {
            Ok(r) => r,
            Err(err) => {
                response_headers
                    .insert("content-type", http::HeaderValue::from_static("text/plain"));
                return HandlerOutput {
                    code: http::StatusCode::INTERNAL_SERVER_ERROR,
                    body: bytes::Bytes::from(
                        format!("Failed to serialize response body: {err}").into_bytes(),
                    ),
                    headers: response_headers,
                };
            }
        };

        let code = match output {
            UntaggedResult::Ok(_) => http::StatusCode::OK,
            UntaggedResult::Err(err) => {
                let custom_error = err.status_code();
                if custom_error == http::StatusCode::OK {
                    // It means a user has implemented ToStatusCode trait for their
                    // type incorrectly. It is a protocol error to return 200 status
                    // code for an error response, as the client will not be able
                    // to "cast" the response body to the correct type.
                    // So, we are reverting it to internal error
                    http::StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    custom_error
                }
            }
        };

        HandlerOutput {
            code,
            body: bytes::Bytes::from(output_serialized),
            headers: response_headers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
enum UntaggedResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> From<Result<T, E>> for UntaggedResult<T, E> {
    fn from(res: Result<T, E>) -> Self {
        match res {
            Ok(v) => UntaggedResult::Ok(v),
            Err(err) => UntaggedResult::Err(err),
        }
    }
}
