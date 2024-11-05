pub use url::{ParseError as UrlParseError, Url};

pub trait Client {
    type Error;

    fn request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> impl std::future::Future<Output = Result<(http::StatusCode, bytes::Bytes), Self::Error>>;
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
                    http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    http::HeaderValue::from_str(&v_str).unwrap(),
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
        Ok(error) => Err(Error::Application(error)),
        Err(e) if status.is_client_error() => Err(Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseError(status, body),
        }),
        Err(_) => Err(Error::Server(status, body)),
    }
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
}
