// Unit tests for transport metadata functionality
// Tests all internal APIs, error variants, and edge cases comprehensively
// Import the transport metadata types directly from reflectapi crate
use reflectapi::rt::{
    ApiResult, Client, Error, ProtocolErrorStage, RequestTiming, TransportMetadata,
    TransportResponse, Url, __request_impl,
};
use std::collections::HashMap;

/// Mock client for testing transport metadata functionality
struct MockClient {
    responses: HashMap<String, Result<TransportResponse, MockError>>,
}

#[derive(Debug)]
struct MockError {
    message: String,
}

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mock error: {}", self.message)
    }
}

impl std::error::Error for MockError {}

impl MockClient {
    fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    fn with_success(
        mut self,
        path: &str,
        status: u16,
        body: &str,
        headers: Vec<(&str, &str)>,
    ) -> Self {
        let started_at = std::time::SystemTime::now();
        let completed_at = started_at + std::time::Duration::from_millis(100);

        let mut header_map = http::HeaderMap::new();
        for (name, value) in headers {
            header_map.insert(
                http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                http::HeaderValue::from_str(value).unwrap(),
            );
        }

        let response = TransportResponse {
            status: http::StatusCode::from_u16(status).unwrap(),
            body: bytes::Bytes::from(body.to_string()),
            headers: header_map,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: HashMap::new(),
        };

        self.responses.insert(path.to_string(), Ok(response));
        self
    }

    fn with_error(
        mut self,
        path: &str,
        status: u16,
        body: &str,
        headers: Vec<(&str, &str)>,
    ) -> Self {
        let started_at = std::time::SystemTime::now();
        let completed_at = started_at + std::time::Duration::from_millis(50);

        let mut header_map = http::HeaderMap::new();
        for (name, value) in headers {
            header_map.insert(
                http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                http::HeaderValue::from_str(value).unwrap(),
            );
        }

        let response = TransportResponse {
            status: http::StatusCode::from_u16(status).unwrap(),
            body: bytes::Bytes::from(body.to_string()),
            headers: header_map,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: HashMap::new(),
        };

        self.responses.insert(path.to_string(), Ok(response));
        self
    }

    fn with_network_error(mut self, path: &str, message: &str) -> Self {
        self.responses.insert(
            path.to_string(),
            Err(MockError {
                message: message.to_string(),
            }),
        );
        self
    }
}

impl Client for MockClient {
    type Error = MockError;

    async fn request(
        &self,
        url: Url,
        _body: bytes::Bytes,
        _headers: http::HeaderMap,
    ) -> Result<TransportResponse, Self::Error> {
        let path = url.path();
        match self.responses.get(path) {
            Some(Ok(response)) => Ok(TransportResponse {
                status: response.status,
                body: response.body.clone(),
                headers: response.headers.clone(),
                timing: response.timing.clone(),
                extensions: HashMap::new(), // Extensions cannot be cloned as they contain Any
            }),
            Some(Err(error)) => Err(MockError {
                message: error.message.clone(),
            }),
            None => Err(MockError {
                message: format!("No response configured for path: {}", path),
            }),
        }
    }
}

#[tokio::test]
async fn test_success_response_with_metadata() {
    let client = MockClient::new().with_success(
        "/test",
        201,
        r#"{"result": "created"}"#,
        vec![
            ("content-type", "application/json"),
            ("x-request-id", "12345"),
            ("cache-control", "no-cache"),
        ],
    );

    let url = "http://localhost:3000/test".parse().unwrap();
    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, serde_json::json!({}), serde_json::json!({})).await;

    assert!(result.is_ok());
    let api_result = result.unwrap();

    // Test the wrapped value
    assert_eq!(api_result.value["result"], "created");

    // Test status code
    assert_eq!(api_result.metadata.status, http::StatusCode::CREATED);

    // Test headers - both presence and values
    assert!(api_result.metadata.headers.contains_key("content-type"));
    assert_eq!(
        api_result.metadata.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        api_result.metadata.headers.get("x-request-id").unwrap(),
        "12345"
    );
    assert_eq!(
        api_result.metadata.headers.get("cache-control").unwrap(),
        "no-cache"
    );

    // Test timing information
    assert!(api_result.metadata.timing.is_some());
    let timing = api_result.metadata.timing.as_ref().unwrap();
    assert!(timing.started_at <= timing.completed_at);
    let duration = timing.duration();
    assert!(duration >= std::time::Duration::from_millis(100));
    assert!(duration < std::time::Duration::from_secs(1));

    // Test Deref functionality for backward compatibility
    let value_via_deref: &serde_json::Value = &api_result;
    assert_eq!(value_via_deref["result"], "created");
}

#[tokio::test]
async fn test_application_error_with_metadata() {
    let client = MockClient::new().with_error(
        "/test",
        400,
        r#"{"type": "ValidationError", "message": "Invalid input"}"#,
        vec![
            ("content-type", "application/json"),
            ("x-error-code", "VAL001"),
            ("retry-after", "30"),
        ],
    );

    let url = "http://localhost:3000/test".parse().unwrap();
    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, serde_json::json!({}), serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match &error {
        Error::Application { error, metadata } => {
            // Test error content
            assert_eq!(error["type"], "ValidationError");
            assert_eq!(error["message"], "Invalid input");

            // Test status code
            assert_eq!(metadata.status, http::StatusCode::BAD_REQUEST);

            // Test headers - both presence and values
            assert_eq!(
                metadata.headers.get("content-type").unwrap(),
                "application/json"
            );
            assert_eq!(metadata.headers.get("x-error-code").unwrap(), "VAL001");
            assert_eq!(metadata.headers.get("retry-after").unwrap(), "30");

            // Test timing information
            assert!(metadata.timing.is_some());
            let timing = metadata.timing.as_ref().unwrap();
            assert!(timing.started_at <= timing.completed_at);
            let duration = timing.duration();
            assert!(duration >= std::time::Duration::from_millis(50));
            assert!(duration < std::time::Duration::from_secs(1));
        }
        _ => panic!("Expected Application error, got: {:?}", error),
    }

    // Test unified metadata access
    let metadata = error.transport_metadata().unwrap();
    assert_eq!(metadata.status, http::StatusCode::BAD_REQUEST);

    // Test backward compatibility method
    assert_eq!(error.status_code().unwrap(), http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_server_error_with_metadata() {
    let client = MockClient::new().with_error(
        "/test",
        500,
        "Internal Server Error",
        vec![
            ("content-type", "text/plain"),
            ("x-incident-id", "INC-789"),
            ("server", "nginx/1.20.1"),
        ],
    );

    let url = "http://localhost:3000/test".parse().unwrap();
    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, serde_json::json!({}), serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match error {
        Error::Server {
            status,
            body,
            metadata,
        } => {
            // Test server error fields
            assert_eq!(status, http::StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, bytes::Bytes::from("Internal Server Error"));

            // Test status code
            assert_eq!(metadata.status, http::StatusCode::INTERNAL_SERVER_ERROR);

            // Test headers - both presence and values
            assert_eq!(metadata.headers.get("content-type").unwrap(), "text/plain");
            assert_eq!(metadata.headers.get("x-incident-id").unwrap(), "INC-789");
            assert_eq!(metadata.headers.get("server").unwrap(), "nginx/1.20.1");

            // Test timing information
            assert!(metadata.timing.is_some());
            let timing = metadata.timing.as_ref().unwrap();
            assert!(timing.started_at <= timing.completed_at);
            let duration = timing.duration();
            assert!(duration >= std::time::Duration::from_millis(50));
        }
        _ => panic!("Expected Server error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_network_error_with_optional_metadata() {
    let client = MockClient::new().with_network_error("/test", "Connection refused");

    let url = "http://localhost:3000/test".parse().unwrap();
    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, serde_json::json!({}), serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match &error {
        Error::Network { error, metadata } => {
            // Test network error content
            assert_eq!(error.message, "Connection refused");

            // Test that metadata is None for network errors (as they don't have HTTP responses)
            assert!(metadata.is_none());
        }
        _ => panic!("Expected Network error, got: {:?}", error),
    }

    // Test unified metadata access returns None for network errors
    assert!(error.transport_metadata().is_none());
    assert!(error.status_code().is_none());
}

#[tokio::test]
async fn test_protocol_error_with_metadata() {
    // Test protocol error by providing invalid JSON in success response
    let client = MockClient::new().with_success(
        "/test",
        200,
        "invalid json {",
        vec![("content-type", "application/json")],
    );

    let url = "http://localhost:3000/test".parse().unwrap();
    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, serde_json::json!({}), serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match error {
        Error::Protocol {
            info,
            stage,
            metadata,
        } => {
            // Test protocol error info
            assert!(info.contains("expected"));

            // Test protocol error stage
            match stage {
                ProtocolErrorStage::DeserializeResponseBody(body) => {
                    assert_eq!(body, bytes::Bytes::from("invalid json {"));
                }
                _ => panic!("Expected DeserializeResponseBody stage, got: {:?}", stage),
            }

            // Test metadata is present for protocol errors during response processing
            assert_eq!(metadata.status, http::StatusCode::OK);
            assert_eq!(
                metadata.headers.get("content-type").unwrap(),
                "application/json"
            );
            assert!(metadata.timing.is_some());
        }
        _ => panic!("Expected Protocol error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_protocol_error_during_serialization() {
    // Create a mock URL that won't be called
    let client = MockClient::new();
    let url = "http://localhost:3000/test".parse().unwrap();

    // Create a type that fails to serialize
    #[derive(Debug)]
    struct FailingSerialize;

    impl serde::Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("Serialization failed"))
        }
    }

    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, MockError>> =
        __request_impl(&client, url, FailingSerialize, serde_json::json!({})).await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match error {
        Error::Protocol {
            info,
            stage,
            metadata,
        } => {
            // Test protocol error info
            assert!(info.contains("Serialization failed"));

            // Test protocol error stage
            match stage {
                ProtocolErrorStage::SerializeRequestBody => {
                    // Expected stage for request body serialization failure
                }
                _ => panic!("Expected SerializeRequestBody stage, got: {:?}", stage),
            }

            // Test that metadata has default values for pre-request errors
            assert_eq!(metadata.status, http::StatusCode::BAD_REQUEST);
            assert!(metadata.headers.is_empty());
            assert!(metadata.timing.is_none());
        }
        _ => panic!("Expected Protocol error, got: {:?}", error),
    }
}

#[tokio::test]
async fn test_all_error_variants_have_transport_metadata_access() {
    // Test that all error variants can be accessed via the unified interface
    let app_error: Error<serde_json::Value, MockError> = Error::Application {
        error: serde_json::json!({"type": "test"}),
        metadata: TransportMetadata {
            status: http::StatusCode::BAD_REQUEST,
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: HashMap::new(),
        },
    };

    let net_error_with_metadata: Error<serde_json::Value, MockError> = Error::Network {
        error: MockError {
            message: "test".to_string(),
        },
        metadata: Some(TransportMetadata {
            status: http::StatusCode::BAD_GATEWAY,
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: HashMap::new(),
        }),
    };

    let net_error_without_metadata: Error<serde_json::Value, MockError> = Error::Network {
        error: MockError {
            message: "test".to_string(),
        },
        metadata: None,
    };

    let protocol_error: Error<serde_json::Value, MockError> = Error::Protocol {
        info: "test".to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
        metadata: TransportMetadata {
            status: http::StatusCode::BAD_REQUEST,
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: HashMap::new(),
        },
    };

    let server_error: Error<serde_json::Value, MockError> = Error::Server {
        status: http::StatusCode::INTERNAL_SERVER_ERROR,
        body: bytes::Bytes::from("test"),
        metadata: TransportMetadata {
            status: http::StatusCode::INTERNAL_SERVER_ERROR,
            headers: http::HeaderMap::new(),
            timing: None,
            extensions: HashMap::new(),
        },
    };

    // Test unified metadata access
    assert_eq!(
        app_error.transport_metadata().unwrap().status,
        http::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        net_error_with_metadata.transport_metadata().unwrap().status,
        http::StatusCode::BAD_GATEWAY
    );
    assert!(net_error_without_metadata.transport_metadata().is_none());
    assert_eq!(
        protocol_error.transport_metadata().unwrap().status,
        http::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        server_error.transport_metadata().unwrap().status,
        http::StatusCode::INTERNAL_SERVER_ERROR
    );

    // Test backward compatibility status_code method
    assert_eq!(
        app_error.status_code().unwrap(),
        http::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        net_error_with_metadata.status_code().unwrap(),
        http::StatusCode::BAD_GATEWAY
    );
    assert!(net_error_without_metadata.status_code().is_none());
    assert_eq!(
        protocol_error.status_code().unwrap(),
        http::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        server_error.status_code().unwrap(),
        http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn test_transport_metadata_clone() {
    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    let timing = RequestTiming {
        started_at: std::time::SystemTime::now(),
        completed_at: std::time::SystemTime::now() + std::time::Duration::from_millis(100),
    };

    let metadata = TransportMetadata {
        status: http::StatusCode::OK,
        headers: headers.clone(),
        timing: Some(timing.clone()),
        extensions: HashMap::new(),
    };

    let cloned = metadata.clone();

    assert_eq!(cloned.status, metadata.status);
    assert_eq!(cloned.headers, metadata.headers);
    assert_eq!(cloned.timing, metadata.timing);
    // Extensions are always empty after clone (due to Any trait objects)
    assert!(cloned.extensions.is_empty());
}

#[test]
fn test_request_timing_duration() {
    let start = std::time::SystemTime::now();
    let end = start + std::time::Duration::from_millis(150);

    let timing = RequestTiming {
        started_at: start,
        completed_at: end,
    };

    let duration = timing.duration();
    assert_eq!(duration, std::time::Duration::from_millis(150));

    // Test edge case where completed_at is before started_at (shouldn't happen in practice)
    let backwards_timing = RequestTiming {
        started_at: end,
        completed_at: start,
    };

    let backwards_duration = backwards_timing.duration();
    assert_eq!(backwards_duration, std::time::Duration::ZERO);
}

#[test]
fn test_api_result_methods() {
    let metadata = TransportMetadata {
        status: http::StatusCode::CREATED,
        headers: http::HeaderMap::new(),
        timing: None,
        extensions: HashMap::new(),
    };

    let api_result = ApiResult {
        value: "test_value".to_string(),
        metadata: metadata.clone(),
    };

    // Test into_inner method
    let inner = api_result.into_inner();
    assert_eq!(inner, "test_value");

    // Test metadata access and Deref (need new instance since into_inner consumed the first)
    let api_result2 = ApiResult {
        value: "test_value2".to_string(),
        metadata,
    };

    assert_eq!(api_result2.metadata().status, http::StatusCode::CREATED);
    assert_eq!(&api_result2 as &str, "test_value2"); // Test Deref
}
