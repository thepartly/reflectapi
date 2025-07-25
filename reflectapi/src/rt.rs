pub use url::{ParseError as UrlParseError, Url};

/// Captures request-response timing information
#[derive(Debug, Clone, PartialEq)]
pub struct RequestTiming {
    pub started_at: std::time::SystemTime,
    pub completed_at: std::time::SystemTime,
}

impl RequestTiming {
    pub fn duration(&self) -> std::time::Duration {
        self.completed_at
            .duration_since(self.started_at)
            .unwrap_or(std::time::Duration::ZERO)
    }
}

/// Rich transport metadata for requests and responses
#[derive(Debug)]
pub struct TransportMetadata {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub timing: Option<RequestTiming>,
    /// Generic storage for transport-specific data (e.g., raw reqwest::Response)
    pub extensions: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl Clone for TransportMetadata {
    fn clone(&self) -> Self {
        TransportMetadata {
            status: self.status,
            headers: self.headers.clone(),
            timing: self.timing.clone(),
            // Extensions cannot be cloned as they contain Any trait objects
            // Create a new empty HashMap instead
            extensions: std::collections::HashMap::new(),
        }
    }
}

/// Enhanced response type that includes full transport metadata
pub struct TransportResponse {
    pub status: http::StatusCode,
    pub body: bytes::Bytes,
    pub headers: http::HeaderMap,
    pub timing: Option<RequestTiming>,
    pub extensions: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

/// Wraps successful API responses with transport metadata
#[derive(Debug)]
pub struct ApiResult<T> {
    pub value: T,
    pub metadata: TransportMetadata,
}

impl<T> ApiResult<T> {
    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn metadata(&self) -> &TransportMetadata {
        &self.metadata
    }
}

/// Provides ergonomic, backward-compatible-like access to the inner value
impl<T> std::ops::Deref for ApiResult<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for ApiResult<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub trait Client {
    type Error;

    fn request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> impl std::future::Future<Output = Result<TransportResponse, Self::Error>>;
}

pub enum Error<AE, NE> {
    Application {
        error: AE,
        metadata: TransportMetadata,
    },
    Network {
        error: NE,
        metadata: Option<TransportMetadata>, // Metadata might not exist for network errors
    },
    Protocol {
        info: String,
        stage: ProtocolErrorStage,
        metadata: TransportMetadata,
    },
    Server {
        status: http::StatusCode,
        body: bytes::Bytes,
        metadata: TransportMetadata,
    },
}

impl<AE: core::fmt::Debug, NE: core::fmt::Debug> core::fmt::Debug for Error<AE, NE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Application { error, .. } => write!(f, "application error: {error:?}"),
            Error::Network { error, .. } => write!(f, "network error: {error:?}"),
            Error::Protocol { info, stage, .. } => write!(f, "protocol error: {info} at {stage:?}"),
            Error::Server { status, body, .. } => write!(
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
            Error::Application { error, .. } => write!(f, "application error: {error}"),
            Error::Network { error, .. } => write!(f, "network error: {error}"),
            Error::Protocol { info, stage, .. } => write!(f, "protocol error: {info} at {stage}"),
            Error::Server { status, body, .. } => write!(
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
            Error::Application { error, .. } => Some(error),
            Error::Network { error, .. } => Some(error),
            Error::Protocol { .. } => None,
            Error::Server { .. } => None,
        }
    }
}

impl<AE, NE> Error<AE, NE> {
    /// Provides unified access to metadata across all error types
    pub fn transport_metadata(&self) -> Option<&TransportMetadata> {
        match self {
            Error::Application { metadata, .. } => Some(metadata),
            Error::Network { metadata, .. } => metadata.as_ref(),
            Error::Protocol { metadata, .. } => Some(metadata),
            Error::Server { metadata, .. } => Some(metadata),
        }
    }

    /// Access HTTP status code (backward compatibility)
    pub fn status_code(&self) -> Option<http::StatusCode> {
        self.transport_metadata().map(|m| m.status)
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
) -> Result<ApiResult<O>, Error<E, C::Error>>
where
    C: Client,
    I: serde::Serialize,
    H: serde::Serialize,
    O: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned,
{
    // Serialize request body
    let body_bytes = serde_json::to_vec(&body).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
        // We don't have metadata yet for serialization errors
        metadata: TransportMetadata {
            status: http::StatusCode::BAD_REQUEST, // Default for protocol errors
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: std::collections::HashMap::new(),
        },
    })?;
    let body_bytes = bytes::Bytes::from(body_bytes);

    // Serialize request headers
    let headers_value = serde_json::to_value(&headers).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestHeaders,
        metadata: TransportMetadata {
            status: http::StatusCode::BAD_REQUEST,
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: std::collections::HashMap::new(),
        },
    })?;

    let mut header_map = http::HeaderMap::new();
    match headers_value {
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
                metadata: TransportMetadata {
                    status: http::StatusCode::BAD_REQUEST,
                    headers: http::HeaderMap::new(),
                    timing: None,
                    extensions: std::collections::HashMap::new(),
                },
            });
        }
    }

    // Make the transport request
    let transport_response = client
        .request(url, body_bytes, header_map)
        .await
        .map_err(|e| Error::Network {
            error: e,
            metadata: None, // Network errors may not have response metadata
        })?;

    // Create transport metadata from response
    let metadata = TransportMetadata {
        status: transport_response.status,
        headers: transport_response.headers.clone(),
        timing: transport_response.timing,
        extensions: transport_response.extensions,
    };

    // Handle successful responses
    if transport_response.status.is_success() {
        let output =
            serde_json::from_slice(&transport_response.body).map_err(|e| Error::Protocol {
                info: e.to_string(),
                stage: ProtocolErrorStage::DeserializeResponseBody(transport_response.body.clone()),
                metadata: metadata.clone(),
            })?;

        return Ok(ApiResult {
            value: output,
            metadata,
        });
    }

    // Handle error responses based on status code
    match serde_json::from_slice::<E>(&transport_response.body) {
        Ok(error) if !transport_response.status.is_server_error() => {
            Err(Error::Application { error, metadata })
        }
        Err(e) if transport_response.status.is_client_error() => Err(Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseError(
                transport_response.status,
                transport_response.body,
            ),
            metadata,
        }),
        _ => Err(Error::Server {
            status: transport_response.status,
            body: transport_response.body,
            metadata,
        }),
    }
}

#[cfg(feature = "reqwest")]
impl Client for reqwest::Client {
    type Error = reqwest::Error;

    async fn request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<TransportResponse, Self::Error> {
        let started_at = std::time::SystemTime::now();

        let response = self.post(url).headers(headers).body(body).send().await?;

        let completed_at = std::time::SystemTime::now();
        let status = response.status();
        let response_headers = response.headers().clone();

        let body = response.bytes().await?;

        Ok(TransportResponse {
            status,
            body,
            headers: response_headers,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: std::collections::HashMap::new(),
        })
    }
}

#[cfg(feature = "reqwest-middleware")]
impl Client for reqwest_middleware::ClientWithMiddleware {
    type Error = reqwest_middleware::Error;

    async fn request(
        &self,
        url: Url,
        body: bytes::Bytes,
        headers: http::HeaderMap,
    ) -> Result<TransportResponse, Self::Error> {
        let started_at = std::time::SystemTime::now();

        let response = self.post(url).headers(headers).body(body).send().await?;

        let completed_at = std::time::SystemTime::now();
        let status = response.status();
        let response_headers = response.headers().clone();

        let body = response.bytes().await?;

        Ok(TransportResponse {
            status,
            body,
            headers: response_headers,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: std::collections::HashMap::new(),
        })
    }
}
