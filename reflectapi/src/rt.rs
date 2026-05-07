use std::{future::Future, pin::Pin};

use futures_util::Stream;
pub use url::{ParseError as UrlParseError, Url};

pub fn error_to_string<T: serde::Serialize>(error: &T) -> String {
    serde_json::to_string(error).unwrap_or_else(|e| format!("Failed to serialize error: {e}"))
}

pub trait Client {
    type Error;

    fn request(
        &self,
        request: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse<Self::Error>, Self::Error>>;
}

pub struct ClientRequest {
    pub url: Url,
    pub method: http::Method,
    pub headers: http::HeaderMap,
    pub body: bytes::Bytes,
}

impl ClientRequest {
    pub fn path(&self) -> &str {
        self.url.path()
    }
}

#[allow(clippy::type_complexity)]
pub struct ClientResponse<E> {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, E>> + Send + 'static>>,
}

pub enum Error<AE, NE> {
    Application(AE),
    Network(NE),
    Protocol {
        info: String,
        stage: ProtocolErrorStage,
    },
    Server(http::StatusCode, bytes::Bytes),
}

impl<AE: core::fmt::Debug, NE: core::fmt::Debug> core::fmt::Debug for Error<AE, NE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Application(err) => write!(f, "application error: {err:?}"),
            Error::Network(err) => write!(f, "network error: {err:?}"),
            Error::Protocol { info, stage } => write!(f, "protocol error: {info} at {stage:?}"),
            Error::Server(status, body) => write!(
                f,
                "server error: {status} with body: {}",
                String::from_utf8_lossy(body)
            ),
        }
    }
}

impl<AE: core::fmt::Display, NE: core::fmt::Display> core::fmt::Display for Error<AE, NE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Application(err) => write!(f, "application error: {err}"),
            Error::Network(err) => write!(f, "network error: {err}"),
            Error::Protocol { info, stage } => write!(f, "protocol error: {info} at {stage}"),
            Error::Server(status, body) => write!(
                f,
                "server error: {status} with body: {}",
                String::from_utf8_lossy(body)
            ),
        }
    }
}

impl<AE: std::error::Error + 'static, NE: std::error::Error + 'static> std::error::Error
    for Error<AE, NE>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Application(err) => Some(err),
            Error::Network(err) => Some(err),
            Error::Protocol { .. } => None,
            Error::Server(_, _) => None,
        }
    }
}

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

/// Error type for individual stream items.
///
/// Unlike [`Error`], this does not include an `Application` variant because
/// application-level errors can only occur during the initial request/response
/// cycle (stream creation), not per-item during streaming.
pub enum StreamItemError<NE> {
    Network(NE),
    Protocol {
        info: String,
        stage: ProtocolErrorStage,
    },
}

impl<NE: core::fmt::Debug> core::fmt::Debug for StreamItemError<NE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StreamItemError::Network(err) => write!(f, "network error: {err:?}"),
            StreamItemError::Protocol { info, stage } => {
                write!(f, "protocol error: {info} at {stage:?}")
            }
        }
    }
}

impl<NE: core::fmt::Display> core::fmt::Display for StreamItemError<NE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StreamItemError::Network(err) => write!(f, "network error: {err}"),
            StreamItemError::Protocol { info, stage } => {
                write!(f, "protocol error: {info} at {stage}")
            }
        }
    }
}

impl<NE: std::error::Error + 'static> std::error::Error for StreamItemError<NE> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StreamItemError::Network(err) => Some(err),
            StreamItemError::Protocol { .. } => None,
        }
    }
}

pub type StreamResponse<T, AE, NE> =
    Result<BoxStream<Result<T, StreamItemError<NE>>>, Error<AE, NE>>;

pub enum ProtocolErrorStage {
    SerializeRequestBody,
    SerializeRequestHeaders,
    DeserializeResponseBody(bytes::Bytes),
    DeserializeResponseError(http::StatusCode, bytes::Bytes),
}

impl core::fmt::Display for ProtocolErrorStage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolErrorStage::SerializeRequestBody => {
                write!(f, "failed to serialize request body")
            }
            ProtocolErrorStage::SerializeRequestHeaders => {
                write!(f, "failed to serialize request headers")
            }
            ProtocolErrorStage::DeserializeResponseBody(body) => write!(
                f,
                "failed to deserialize response body: {}",
                String::from_utf8_lossy(body)
            ),
            ProtocolErrorStage::DeserializeResponseError(status, body) => write!(
                f,
                "failed to deserialize response error: {} with body: {}",
                status,
                String::from_utf8_lossy(body)
            ),
        }
    }
}

impl core::fmt::Debug for ProtocolErrorStage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtocolErrorStage::SerializeRequestBody => write!(f, "SerializeRequestBody"),
            ProtocolErrorStage::SerializeRequestHeaders => write!(f, "SerializeRequestHeaders"),
            ProtocolErrorStage::DeserializeResponseBody(body) => write!(
                f,
                "DeserializeResponseBody({:?})",
                String::from_utf8_lossy(body)
            ),
            ProtocolErrorStage::DeserializeResponseError(status, body) => write!(
                f,
                "DeserializeResponseError({status}, {:?})",
                String::from_utf8_lossy(body)
            ),
        }
    }
}

#[doc(hidden)]
pub async fn __request_impl<C, I, H, O, E>(
    client: &C,
    url: Url,
    body: I,
    headers: H,
) -> Result<O, Error<E, C::Error>>
where
    C: Client,
    I: serde::Serialize,
    H: serde::Serialize,
    O: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned,
{
    let body = serde_json::to_vec(&body).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
    })?;
    let body = bytes::Bytes::from(body);
    let headers = serde_json::to_value(&headers).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestHeaders,
    })?;

    let mut header_map = http::HeaderMap::new();
    match headers {
        serde_json::Value::Object(headers) => {
            for (k, v) in headers.into_iter() {
                let v_str = match v {
                    serde_json::Value::String(v) => v,
                    v => v.to_string(),
                };
                header_map.insert(
                    http::HeaderName::from_bytes(k.as_bytes()).map_err(|err| Error::Protocol {
                        info: err.to_string(),
                        stage: ProtocolErrorStage::SerializeRequestHeaders,
                    })?,
                    http::HeaderValue::from_str(&v_str).map_err(|err| Error::Protocol {
                        info: err.to_string(),
                        stage: ProtocolErrorStage::SerializeRequestHeaders,
                    })?,
                );
            }
        }
        serde_json::Value::Null => {}
        _ => {
            return Err(Error::Protocol {
                info: "Headers must be an object".to_string(),
                stage: ProtocolErrorStage::SerializeRequestHeaders,
            });
        }
    }

    let response = client
        .request(ClientRequest {
            url,
            method: http::Method::POST,
            headers: header_map,
            body,
        })
        .await
        .map_err(Error::Network)?;
    let status = response.status;
    let body = __collect_byte_stream(response.body)
        .await
        .map_err(Error::Network)?;

    if status.is_success() {
        let output = serde_json::from_slice(&body).map_err(|e| Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseBody(body),
        })?;
        return Ok(output);
    }
    match serde_json::from_slice::<E>(&body) {
        Ok(error) if !status.is_server_error() => Err(Error::Application(error)),
        Err(e) if status.is_client_error() => Err(Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseError(status, body),
        }),
        _ => Err(Error::Server(status, body)),
    }
}

#[cfg(feature = "rt-sse")]
fn __serialize_headers_for_stream<H: serde::Serialize>(
    headers: H,
) -> Result<http::HeaderMap, (String, ProtocolErrorStage)> {
    let headers = serde_json::to_value(&headers)
        .map_err(|e| (e.to_string(), ProtocolErrorStage::SerializeRequestHeaders))?;

    let mut header_map = http::HeaderMap::new();
    header_map.insert(
        http::header::ACCEPT,
        http::HeaderValue::from_static("text/event-stream"),
    );
    match headers {
        serde_json::Value::Object(headers) => {
            for (k, v) in headers.into_iter() {
                let v_str = match v {
                    serde_json::Value::String(v) => v,
                    v => v.to_string(),
                };
                header_map.insert(
                    http::HeaderName::from_bytes(k.as_bytes()).map_err(|err| {
                        (err.to_string(), ProtocolErrorStage::SerializeRequestHeaders)
                    })?,
                    http::HeaderValue::from_str(&v_str).map_err(|err| {
                        (err.to_string(), ProtocolErrorStage::SerializeRequestHeaders)
                    })?,
                );
            }
        }
        serde_json::Value::Null => {}
        _ => {
            return Err((
                "Headers must be an object".to_string(),
                ProtocolErrorStage::SerializeRequestHeaders,
            ));
        }
    }
    Ok(header_map)
}

#[doc(hidden)]
#[cfg(feature = "rt-sse")]
pub async fn __stream_request_impl<C, I, H, O, E>(
    client: &C,
    url: Url,
    body: I,
    headers: H,
) -> Result<BoxStream<Result<O, StreamItemError<C::Error>>>, Error<E, C::Error>>
where
    C: Client,
    C::Error: Send + 'static,
    I: serde::Serialize,
    H: serde::Serialize,
    O: serde::de::DeserializeOwned + Send + 'static,
    E: serde::de::DeserializeOwned + Send + 'static,
{
    use futures_util::StreamExt;
    use sseer::event_stream::EventStream;
    use sseer::json_stream::JsonStream;
    use sseer::{errors::EventStreamError, json_stream::JsonStreamError};

    let body = serde_json::to_vec(&body).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
    })?;
    let body = bytes::Bytes::from(body);
    let header_map = __serialize_headers_for_stream(headers)
        .map_err(|(info, stage)| Error::Protocol { info, stage })?;

    let response = client
        .request(ClientRequest {
            url,
            method: http::Method::POST,
            headers: header_map,
            body,
        })
        .await
        .map_err(Error::Network)?;
    let status = response.status;
    let byte_stream = response.body;

    if status.is_success() {
        let event_stream = EventStream::new(byte_stream);
        let json_stream = JsonStream::<O, _>::new_default(event_stream);
        let stream = json_stream.map(|item| {
            item.map_err(|err| match err {
                JsonStreamError::Stream(err) => match err {
                    EventStreamError::Transport(err) => StreamItemError::Network(err),
                    EventStreamError::Utf8Error(err) => StreamItemError::Protocol {
                        info: err.to_string(),
                        stage: ProtocolErrorStage::DeserializeResponseBody(bytes::Bytes::new()),
                    },
                },
                JsonStreamError::Deserialize(err) => StreamItemError::Protocol {
                    info: err.to_string(),
                    stage: ProtocolErrorStage::DeserializeResponseBody(bytes::Bytes::new()),
                },
            })
        });
        return Ok(Box::pin(stream));
    }

    let body = __collect_byte_stream(byte_stream)
        .await
        .map_err(Error::Network)?;
    match serde_json::from_slice::<E>(&body) {
        Ok(error) if !status.is_server_error() => Err(Error::Application(error)),
        Err(e) if status.is_client_error() => Err(Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseError(status, body),
        }),
        _ => Err(Error::Server(status, body)),
    }
}

async fn __collect_byte_stream<E>(
    stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, E>> + Send>>,
) -> Result<bytes::Bytes, E> {
    use futures_util::StreamExt;
    let mut chunks = Vec::new();
    futures_util::pin_mut!(stream);
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk?);
    }
    let total_len = chunks.iter().map(|c| c.len()).sum();
    let mut buf = bytes::BytesMut::with_capacity(total_len);
    for chunk in chunks {
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.freeze())
}

#[cfg(feature = "reqwest")]
impl Client for reqwest::Client {
    type Error = reqwest::Error;

    async fn request(
        &self,
        request: ClientRequest,
    ) -> Result<ClientResponse<Self::Error>, Self::Error> {
        let response = self
            .request(request.method, request.url)
            .headers(request.headers)
            .body(request.body)
            .send()
            .await?;
        Ok(ClientResponse {
            status: response.status(),
            headers: response.headers().clone(),
            body: Box::pin(response.bytes_stream()),
        })
    }
}

#[cfg(feature = "reqwest-middleware")]
impl Client for reqwest_middleware::ClientWithMiddleware {
    type Error = reqwest_middleware::Error;

    async fn request(
        &self,
        request: ClientRequest,
    ) -> Result<ClientResponse<Self::Error>, Self::Error> {
        let response = self
            .request(request.method, request.url)
            .headers(request.headers)
            .body(request.body)
            .send()
            .await?;
        Ok(ClientResponse {
            status: response.status(),
            headers: response.headers().clone(),
            body: Box::pin(futures_util::StreamExt::map(response.bytes_stream(), |r| {
                r.map_err(reqwest_middleware::Error::Reqwest)
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[derive(Clone)]
    struct ShapeClient;

    impl Client for ShapeClient {
        type Error = std::convert::Infallible;

        async fn request(
            &self,
            request: ClientRequest,
        ) -> Result<ClientResponse<Self::Error>, Self::Error> {
            assert_eq!(request.method, http::Method::POST);
            assert_eq!(request.path(), "/shape.test");
            assert_eq!(request.body.as_ref(), br#"{"name":"input"}"#);

            Ok(ClientResponse {
                status: http::StatusCode::OK,
                headers: http::HeaderMap::new(),
                body: Box::pin(stream::once(async {
                    Ok(bytes::Bytes::from_static(br#"{"name":"output"}"#))
                })),
            })
        }
    }

    #[derive(serde::Deserialize, serde::Serialize)]
    struct ShapeRequest {
        name: String,
    }

    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    struct ShapeResponse {
        name: String,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct ShapeError {}

    #[test]
    fn client_request_shape_is_used() {
        let output = futures::executor::block_on(__request_impl::<
            _,
            _,
            crate::Empty,
            ShapeResponse,
            ShapeError,
        >(
            &ShapeClient,
            Url::parse("https://example.com/shape.test").unwrap(),
            ShapeRequest {
                name: "input".to_string(),
            },
            crate::Empty {},
        ))
        .unwrap();

        assert_eq!(
            output,
            ShapeResponse {
                name: "output".to_string()
            }
        );
    }
}
