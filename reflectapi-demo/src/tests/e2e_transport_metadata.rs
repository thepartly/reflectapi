// End-to-end integration tests for transport metadata functionality
// Tests realistic client usage patterns and compatibility with generated clients
use reflectapi::rt::{ApiResult, Client, Error, RequestTiming, TransportResponse};
use std::collections::HashMap;

// Simple mock client to test transport metadata functionality end-to-end
struct TestClient {
    should_fail: bool,
}

impl TestClient {
    fn new(should_fail: bool) -> Self {
        Self { should_fail }
    }
}

#[derive(Debug)]
struct TestError {
    message: String,
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test error: {}", self.message)
    }
}

impl std::error::Error for TestError {}

impl Client for TestClient {
    type Error = TestError;

    async fn request(
        &self,
        _url: reflectapi::url::Url,
        _body: bytes::Bytes,
        _headers: http::HeaderMap,
    ) -> Result<TransportResponse, Self::Error> {
        if self.should_fail {
            return Err(TestError {
                message: "Network connection failed".to_string(),
            });
        }

        let started_at = std::time::SystemTime::now();
        let completed_at = started_at + std::time::Duration::from_millis(50);

        let mut headers = http::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-request-id", "test-12345".parse().unwrap());

        Ok(TransportResponse {
            status: http::StatusCode::OK,
            body: bytes::Bytes::from(r#"{"message": "success", "data": {"id": 42}}"#),
            headers,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: HashMap::new(),
        })
    }
}

#[tokio::test]
async fn test_e2e_successful_response_with_transport_metadata() {
    let client = TestClient::new(false);
    let url = "http://localhost:3000/test".parse().unwrap();

    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, TestError>> =
        reflectapi::rt::__request_impl(&client, url, serde_json::json!({}), serde_json::json!({}))
            .await;

    assert!(result.is_ok());
    let api_result = result.unwrap();

    // Verify the response data
    assert_eq!(api_result.value["message"], "success");
    assert_eq!(api_result.value["data"]["id"], 42);

    // Verify transport metadata
    assert_eq!(api_result.metadata.status, http::StatusCode::OK);
    assert_eq!(
        api_result.metadata.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        api_result.metadata.headers.get("x-request-id").unwrap(),
        "test-12345"
    );

    // Verify timing information
    assert!(api_result.metadata.timing.is_some());
    let timing = api_result.metadata.timing.as_ref().unwrap();
    assert!(timing.started_at <= timing.completed_at);
    assert!(timing.duration().as_millis() >= 50);

    // Test Deref functionality
    let _value: &serde_json::Value = &api_result;
    assert_eq!(&api_result as &serde_json::Value, &api_result.value);
}

#[tokio::test]
async fn test_e2e_network_error_handling() {
    let failing_client = TestClient::new(true);
    let url = "http://localhost:3000/test".parse().unwrap();

    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, TestError>> =
        reflectapi::rt::__request_impl(
            &failing_client,
            url,
            serde_json::json!({}),
            serde_json::json!({}),
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match error {
        Error::Network {
            ref error,
            ref metadata,
        } => {
            assert_eq!(error.message, "Network connection failed");
            assert!(metadata.is_none());

            // Verified in the outer scope below
        }
        other => panic!("Expected Network error, got: {:?}", other),
    }

    // Test unified metadata access (after the match to avoid borrow issues)
    assert!(error.transport_metadata().is_none());
    assert!(error.status_code().is_none());
}

#[tokio::test]
async fn test_e2e_client_compatibility_patterns() {
    // Test that the generated client patterns work with transport metadata
    let client = TestClient::new(false);
    let url = "http://localhost:3000/test".parse().unwrap();

    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, TestError>> =
        reflectapi::rt::__request_impl(&client, url, serde_json::json!({}), serde_json::json!({}))
            .await;

    match result {
        Ok(api_result) => {
            // Test that we can access both the value and metadata
            let value = api_result.into_inner();
            assert_eq!(value["message"], "success");
        }
        Err(error) => {
            // Test error metadata access patterns
            if let Some(metadata) = error.transport_metadata() {
                assert!(metadata.status.is_server_error() || metadata.status.is_client_error());
            }
            panic!("Expected success for this test");
        }
    }
}

// Mock error response test
struct ErrorClient;

impl Client for ErrorClient {
    type Error = TestError;

    async fn request(
        &self,
        _url: reflectapi::url::Url,
        _body: bytes::Bytes,
        _headers: http::HeaderMap,
    ) -> Result<TransportResponse, Self::Error> {
        let started_at = std::time::SystemTime::now();
        let completed_at = started_at + std::time::Duration::from_millis(25);

        let mut headers = http::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-error-code", "ERR001".parse().unwrap());

        Ok(TransportResponse {
            status: http::StatusCode::BAD_REQUEST,
            body: bytes::Bytes::from(
                r#"{"error": "validation_failed", "message": "Invalid input"}"#,
            ),
            headers,
            timing: Some(RequestTiming {
                started_at,
                completed_at,
            }),
            extensions: HashMap::new(),
        })
    }
}

#[tokio::test]
async fn test_e2e_application_error_with_metadata() {
    let client = ErrorClient;
    let url = "http://localhost:3000/test".parse().unwrap();

    let result: Result<ApiResult<serde_json::Value>, Error<serde_json::Value, TestError>> =
        reflectapi::rt::__request_impl(&client, url, serde_json::json!({}), serde_json::json!({}))
            .await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    match error {
        Error::Application {
            ref error,
            ref metadata,
        } => {
            // Verify error content
            assert_eq!(error["error"], "validation_failed");
            assert_eq!(error["message"], "Invalid input");

            // Verify metadata
            assert_eq!(metadata.status, http::StatusCode::BAD_REQUEST);
            assert_eq!(
                metadata.headers.get("content-type").unwrap(),
                "application/json"
            );
            assert_eq!(metadata.headers.get("x-error-code").unwrap(), "ERR001");

            // Verify timing
            assert!(metadata.timing.is_some());
            let timing = metadata.timing.as_ref().unwrap();
            assert!(timing.duration().as_millis() >= 25);
        }
        other => panic!("Expected Application error, got: {:?}", other),
    }

    // Test unified access patterns
    let metadata = error.transport_metadata().unwrap();
    assert_eq!(metadata.status, http::StatusCode::BAD_REQUEST);
    assert_eq!(error.status_code().unwrap(), http::StatusCode::BAD_REQUEST);
}
