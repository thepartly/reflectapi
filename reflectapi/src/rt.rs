use std::{future::Future, pin::Pin};

use futures_util::{Stream, StreamExt};
pub use url::{ParseError as UrlParseError, Url};

pub fn error_to_string<T: serde::Serialize>(error: &T) -> String {
    serde_json::to_string(error).unwrap_or_else(|e| format!("Failed to serialize error: {e}"))
}

pub trait Client {
    type Error;

    fn request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> impl Future<Output = Result<(http::StatusCode, bytes::Bytes), Self::Error>>;

    #[allow(clippy::type_complexity)]
    fn stream_request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> impl Future<
        Output = Result<
            (
                http::StatusCode,
                Pin<Box<dyn Stream<Item = Result<bytes::Bytes, Self::Error>> + Send + 'static>>,
            ),
            Self::Error,
        >,
    >;
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

pub type StreamResponse<T, AE, NE> = Result<BoxStream<Result<T, Error<AE, NE>>>, Error<AE, NE>>;

pub enum ProtocolErrorStage {
    SerializeRequestBody,
    SerializeRequestHeaders,
    DeserializeResponseBody(bytes::Bytes),
    DeserializeResponseError(http::StatusCode, bytes::Bytes),
    DeserializeStreamItem(String),
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
            ProtocolErrorStage::DeserializeStreamItem(data) => {
                write!(f, "failed to deserialize stream item: {data}")
            }
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
            ProtocolErrorStage::DeserializeStreamItem(data) => {
                write!(f, "DeserializeStreamItem({data:?})")
            }
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

    let (status, body) = client
        .request(url, body, header_map)
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

#[doc(hidden)]
pub fn __parse_sse_stream(
    body: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, impl Send + 'static>> + Send>>,
) -> BoxStream<Result<String, String>> {
    let buffer = String::new();
    let data = String::new();

    Box::pin(futures_util::stream::unfold(
        (body, buffer, data),
        |(mut body, mut buffer, mut data)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim_end_matches('\r').to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() {
                        if !data.is_empty() {
                            if data.ends_with('\n') {
                                data.pop();
                            }
                            let event_data = std::mem::take(&mut data);
                            return Some((Ok(event_data), (body, buffer, data)));
                        }
                        continue;
                    }

                    if let Some(value) = line.strip_prefix("data:") {
                        let value = value.strip_prefix(' ').unwrap_or(value);
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(value);
                    }
                    continue;
                }

                match body.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                    }
                    Some(Err(_)) => {
                        return Some((Err("stream error".to_string()), (body, buffer, data)));
                    }
                    None => {
                        if !data.is_empty() {
                            if data.ends_with('\n') {
                                data.pop();
                            }
                            let event_data = std::mem::take(&mut data);
                            return Some((Ok(event_data), (body, buffer, data)));
                        }
                        return None;
                    }
                }
            }
        },
    ))
}

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
pub async fn __stream_request_impl<C, I, H, O, E>(
    client: &C,
    url: Url,
    body: I,
    headers: H,
) -> Result<BoxStream<Result<O, Error<E, C::Error>>>, Error<E, C::Error>>
where
    C: Client,
    C::Error: Send + 'static,
    I: serde::Serialize,
    H: serde::Serialize,
    O: serde::de::DeserializeOwned + Send + 'static,
    E: serde::de::DeserializeOwned + Send + 'static,
{
    let body = serde_json::to_vec(&body).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
    })?;
    let body = bytes::Bytes::from(body);
    let header_map = __serialize_headers_for_stream(headers)
        .map_err(|(info, stage)| Error::Protocol { info, stage })?;

    let (status, byte_stream) = client
        .stream_request(url, body, header_map)
        .await
        .map_err(Error::Network)?;

    if status.is_success() {
        let sse_stream = __parse_sse_stream(byte_stream);
        let mapped = futures_util::StreamExt::map(sse_stream, |item| match item {
            Ok(data) => serde_json::from_str::<O>(&data).map_err(|e| Error::Protocol {
                info: e.to_string(),
                stage: ProtocolErrorStage::DeserializeStreamItem(data),
            }),
            Err(e) => Err(Error::Protocol {
                info: e,
                stage: ProtocolErrorStage::DeserializeStreamItem(String::new()),
            }),
        });
        return Ok(Box::pin(mapped));
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
        path: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<(http::StatusCode, bytes::Bytes), Self::Error> {
        let response = self.post(path).headers(headers).body(body).send().await?;
        let status = response.status();
        let body = response.bytes().await?;
        Ok((status, body))
    }

    async fn stream_request(
        &self,
        path: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<
        (
            http::StatusCode,
            Pin<Box<dyn Stream<Item = Result<bytes::Bytes, Self::Error>> + Send + 'static>>,
        ),
        Self::Error,
    > {
        let response = self.post(path).headers(headers).body(body).send().await?;
        let status = response.status();
        Ok((status, Box::pin(response.bytes_stream())))
    }
}

#[cfg(feature = "reqwest-middleware")]
impl Client for reqwest_middleware::ClientWithMiddleware {
    type Error = reqwest_middleware::Error;

    async fn request(
        &self,
        path: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<(http::StatusCode, bytes::Bytes), Self::Error> {
        let response = self.post(path).headers(headers).body(body).send().await?;
        let status = response.status();
        let body = response.bytes().await?;
        Ok((status, body))
    }

    async fn stream_request(
        &self,
        path: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<
        (
            http::StatusCode,
            Pin<Box<dyn Stream<Item = Result<bytes::Bytes, Self::Error>> + Send + 'static>>,
        ),
        Self::Error,
    > {
        let response = self.post(path).headers(headers).body(body).send().await?;
        let status = response.status();
        Ok((
            status,
            Box::pin(futures_util::StreamExt::map(response.bytes_stream(), |r| {
                r.map_err(reqwest_middleware::Error::Reqwest)
            })),
        ))
    }
}
