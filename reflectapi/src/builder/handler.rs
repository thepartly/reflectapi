use core::fmt;
use std::{future::Future, pin::Pin, sync::Arc};

use futures_util::{stream::Stream, StreamExt, TryStreamExt};

use http::HeaderName;

use crate::{Function, Input, Output, OutputType, Schema, Struct};

use super::RouteBuilder;

pub struct HandlerInput {
    pub body: bytes::Bytes,
    pub headers: http::HeaderMap,
}

pub(crate) struct HandlerOutput {
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

impl ContentType {
    #[doc(hidden)]
    // Used by tests currently
    pub fn extract(headers: &http::HeaderMap) -> Result<Self, String> {
        match headers.get("content-type") {
            Some(v) => {
                let mime = v
                    .to_str()
                    .map_err(|err| err.to_string())?
                    .parse::<mime::Mime>()
                    .map_err(|err| err.to_string())?;
                if let Some(charset) = mime.get_param(mime::CHARSET) {
                    if charset != mime::UTF_8 {
                        return Err(format!("unsupported charset: {charset}"));
                    }
                }

                if mime.type_() != mime::APPLICATION {
                    return Err(format!("unsupported media type: {mime}"));
                }

                match mime.subtype() {
                    mime::JSON => Ok(ContentType::Json),
                    #[cfg(feature = "msgpack")]
                    mime::MSGPACK => Ok(ContentType::MessagePack),
                    _ => Err(format!("unsupported content-type: {mime}")),
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

pub(crate) type HandlerFuture = Pin<Box<dyn Future<Output = HandlerOutput> + Send + 'static>>;

pub(crate) type HandlerStream =
    Pin<Box<dyn Stream<Item = Result<String, String>> + Send + 'static>>;

pub(crate) enum HandlerCallback<S> {
    Future(Arc<dyn Fn(S, HandlerInput) -> HandlerFuture + Send + Sync>),
    Stream(Arc<dyn Fn(S, HandlerInput) -> Result<HandlerStream, HandlerOutput> + Send + Sync>),
}

impl<S> Clone for HandlerCallback<S>
where
    S: Send + 'static,
{
    fn clone(&self) -> Self {
        match self {
            HandlerCallback::Future(cb) => HandlerCallback::Future(cb.clone()),
            HandlerCallback::Stream(cb) => HandlerCallback::Stream(cb.clone()),
        }
    }
}

pub(crate) struct Handler<S>
where
    S: Send + 'static,
{
    pub name: String,
    pub path: String,
    pub readonly: bool,
    pub input_headers: Vec<HeaderName>,
    pub callback: HandlerCallback<S>,
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
            .finish_non_exhaustive()
    }
}

impl<S> Handler<S>
where
    S: Send + 'static,
{
    pub(crate) fn new<F, Fut, R, I, O, E, H>(
        rb: RouteBuilder,
        handler: F,
        schema: &mut Schema,
    ) -> Handler<S>
    where
        F: Fn(S, I, H) -> Fut + Send + Sync + Copy + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: crate::IntoResult<O, E> + 'static,
        I: Input + serde::de::DeserializeOwned + Send + 'static,
        H: Input + serde::de::DeserializeOwned + Send + 'static,
        O: Output + serde::ser::Serialize + Send + 'static,
        E: Output + serde::ser::Serialize + crate::StatusCode + Send + 'static,
    {
        let (function_def, mut input_headers) =
            Self::mk_function::<I, H, O, E, crate::Empty>(&rb, schema, false);
        schema.functions.push(function_def);

        // inject system header requirements used by the handler wrapper
        input_headers.push(http::header::CONTENT_TYPE);
        input_headers.push(HeaderName::from_static("traceparent"));

        Handler {
            name: rb.name,
            path: rb.path,
            readonly: rb.readonly,
            input_headers,
            callback: HandlerCallback::Future(Arc::new(move |state: S, input: HandlerInput| {
                Box::pin(Self::handler_wrap(state, input, handler)) as _
            })),
        }
    }

    pub(crate) fn new_stream<F, St, R, I, O, E1, E2, H>(
        rb: RouteBuilder,
        handler: F,
        schema: &mut Schema,
    ) -> Handler<S>
    where
        F: Fn(S, I, H) -> Result<St, E1> + Send + Sync + Copy + 'static,
        St: Stream<Item = R> + Send + 'static,
        R: crate::IntoResult<O, E2> + 'static,
        I: Input + serde::de::DeserializeOwned + Send + 'static,
        H: Input + serde::de::DeserializeOwned + Send + 'static,
        O: Output + serde::ser::Serialize + Send + 'static,
        E1: Output + serde::ser::Serialize + crate::StatusCode + Send + 'static,
        E2: Output + serde::ser::Serialize + Send + 'static,
        S: Send + 'static,
    {
        let (function_def, mut input_headers) =
            Self::mk_function::<I, H, O, E1, E2>(&rb, schema, true);
        schema.functions.push(function_def);

        // inject system header requirements used by the handler wrapper
        input_headers.push(http::header::CONTENT_TYPE);
        input_headers.push(HeaderName::from_static("traceparent"));

        Handler {
            name: rb.name,
            path: rb.path,
            readonly: rb.readonly,
            input_headers,
            callback: HandlerCallback::Stream(Arc::new(move |state: S, input: HandlerInput| {
                Self::stream_handler_wrap(state, input, handler)
                    .map(|stream| Box::pin(stream) as HandlerStream)
            })),
        }
    }

    fn mk_function<I: Input, H: Input, O: Output, E1: Output, E2: Output>(
        rb: &RouteBuilder,
        schema: &mut Schema,
        is_stream: bool,
    ) -> (Function, Vec<HeaderName>) {
        let input_type = I::reflectapi_input_type(&mut schema.input_types);
        let output_type = O::reflectapi_output_type(&mut schema.output_types);
        let error_type = E1::reflectapi_output_type(&mut schema.output_types);
        let stream_error_type = E2::reflectapi_output_type(&mut schema.output_types);
        let input_headers = H::reflectapi_input_type(&mut schema.input_types);

        let input_headers_names = schema
            .input_types
            .get_type(input_headers.name.as_str())
            .map(|type_def| match type_def {
                crate::Type::Struct(Struct { fields, .. }) => fields
                    .iter()
                    .map(|field| {
                        let header_name = if field.serde_name.is_empty() {
                            field.name.as_str()
                        } else {
                            field.serde_name.as_str()
                        };
                        HeaderName::from_bytes(header_name.as_bytes())
                            .unwrap_or_else(|_| panic!("invalid header name: `{header_name}`"))
                    })
                    .collect(),
                _ => vec![],
            })
            .unwrap_or_default();
        let f = Function {
            name: rb.name.clone(),
            path: rb.path.clone(),
            deprecation_note: rb.deprecation_note.clone(),
            description: rb.description.clone(),
            input_type: if input_type.name == "reflectapi::Empty" {
                None
            } else {
                Some(input_type)
            },
            output_type: if is_stream {
                OutputType::Stream {
                    item_type: output_type,
                    error_type: if stream_error_type.name == "reflectapi::Infallible" {
                        None
                    } else {
                        Some(stream_error_type)
                    },
                }
            } else {
                OutputType::Single(if output_type.name == "reflectapi::Empty" {
                    None
                } else {
                    Some(output_type)
                })
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
            serialization: vec![
                crate::SerializationMode::Json,
                #[cfg(feature = "msgpack")]
                crate::SerializationMode::Msgpack,
            ],
            readonly: rb.readonly,
            tags: rb.tags.clone(),
        };

        (f, input_headers_names)
    }

    fn parse_input<I: serde::de::DeserializeOwned, H: serde::de::DeserializeOwned>(
        mut handler_input: HandlerInput,
    ) -> Result<(I, H, ContentType, http::HeaderMap), HandlerOutput> {
        let content_type = match ContentType::extract(&handler_input.headers) {
            Ok(t) => t,
            Err(err) => {
                return Err(HandlerOutput {
                    code: http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    body: err.into(),
                    headers: Default::default(),
                });
            }
        };

        let input_parsed = match content_type {
            ContentType::Json => if !handler_input.body.is_empty() {
                serde_json::from_slice::<I>(handler_input.body.as_ref())
            } else {
                serde_json::from_value::<I>(serde_json::Value::Object(serde_json::Map::new()))
            }
            .map_err(|err| err.to_string()),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => if !handler_input.body.is_empty() {
                rmp_serde::from_read::<_, I>(handler_input.body.as_ref())
            } else {
                rmp_serde::from_slice::<I>(&[0x80])
            }
            .map_err(|err| err.to_string()),
        };

        let input = match input_parsed {
            Ok(r) => r,
            Err(err) => {
                return Err(HandlerOutput {
                    code: http::StatusCode::BAD_REQUEST,
                    body: bytes::Bytes::from(
                        format!(
                            "Failed to parse request body: {err}, received: {:?}",
                            handler_input.body
                        )
                        .into_bytes(),
                    ),
                    headers: Default::default(),
                });
            }
        };

        let mut response_headers = http::HeaderMap::new();
        // Respond in the same format as the request
        response_headers.insert("content-type", content_type.into());

        let mut headers_as_json_map = serde_json::Map::new();
        let mut current_header_name = None;
        for (header_name, header_value) in handler_input.headers.drain() {
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
                return Err(HandlerOutput {
                    code: http::StatusCode::BAD_REQUEST,
                    body: bytes::Bytes::from(
                        format!("Failed to parse request headers: {}", err).into_bytes(),
                    ),
                    headers: Default::default(),
                });
            }
        };

        Ok((input, input_headers, content_type, response_headers))
    }

    async fn handler_wrap<F, Fut, R, I, H, O, E>(
        state: S,
        input: HandlerInput,
        handler: F,
    ) -> HandlerOutput
    where
        I: Input + serde::de::DeserializeOwned,
        H: Input + serde::de::DeserializeOwned,
        O: Output + serde::ser::Serialize,
        E: Output + serde::ser::Serialize + crate::StatusCode,
        F: Fn(S, I, H) -> Fut,
        Fut: Future<Output = R> + Send + 'static,
        R: crate::IntoResult<O, E>,
    {
        let (input, input_headers, content_type, mut response_headers) =
            match Self::parse_input::<I, H>(input) {
                Ok(r) => r,
                Err(err) => return err,
            };
        let output = handler(state, input, input_headers).await;
        let output = UntaggedResult::from(output.into_result());

        let output_serialized = match content_type {
            ContentType::Json => serde_json::to_vec(&output).map_err(|err| err.to_string()),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => {
                rmp_serde::to_vec_named(&output).map_err(|err| err.to_string())
            }
        };

        let output_serialized = match output_serialized {
            Ok(r) => r,
            Err(err) => {
                response_headers.insert(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static("text/plain"),
                );
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

    fn stream_handler_wrap<F, St, R, I, H, O, E1, E2>(
        state: S,
        input: HandlerInput,
        handler: F,
    ) -> Result<impl Stream<Item = Result<String, String>>, HandlerOutput>
    where
        I: Input + serde::de::DeserializeOwned,
        H: Input + serde::de::DeserializeOwned,
        O: Output + serde::ser::Serialize,
        E1: Output + serde::ser::Serialize + crate::StatusCode,
        E2: Output + serde::ser::Serialize,
        F: Fn(S, I, H) -> Result<St, E1>,
        St: Stream<Item = R> + Send + 'static,
        R: crate::IntoResult<O, E2>,
    {
        // TODO how do headers work with sse
        let (input, input_headers, content_type, mut response_headers) =
            Self::parse_input::<I, H>(input)?;

        if content_type != ContentType::Json {
            response_headers.insert(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain"),
            );
            return Err(HandlerOutput {
                code: http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                body: bytes::Bytes::from(
                    "Streaming is only supported with application/json content-type",
                ),
                headers: response_headers,
            });
        }

        let st = handler(state, input, input_headers).map_err(|err| HandlerOutput {
            code: err.status_code(),
            body: serde_json::to_vec(&err).unwrap().into(),
            headers: response_headers,
        })?;

        Ok(st.map(Ok).and_then(move |res| {
            let res = res.into_result();
            async move {
                let res = UntaggedResult::from(res);
                serde_json::to_string(&res).map_err(|err| err.to_string())
            }
        }))
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
