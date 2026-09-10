//! `#[reflectapi(header)]`: a response field sent as a header rather than in
//! the body, without the schema — and so every generated client — changing.

use std::sync::Arc;

use axum::body::Body;
use http::{header, Request, StatusCode};
use tower::ServiceExt;

#[derive(reflectapi::Input, serde::Deserialize)]
struct SignInRequest {
    email: String,
}

#[derive(reflectapi::Output, serde::Serialize)]
struct SignedIn {
    user_id: String,
    #[reflectapi(header)]
    #[serde(skip_serializing)]
    set_cookie: Vec<String>,
    /// Renamed, because a header name is not always a field name.
    #[reflectapi(header)]
    #[serde(skip_serializing, rename = "x-request-id")]
    request_id: Option<String>,
}

#[derive(reflectapi::Output, serde::Serialize)]
struct SignInError {
    reason: String,
    #[reflectapi(header)]
    #[serde(skip_serializing)]
    set_cookie: Option<String>,
}

impl reflectapi::StatusCode for SignInError {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

/// A response with no header fields at all, to compare the schema against.
#[derive(reflectapi::Output, serde::Serialize)]
struct SignedInPlain {
    user_id: String,
}

async fn sign_in(
    _: Arc<()>,
    request: SignInRequest,
    _: reflectapi::Empty,
) -> Result<SignedIn, SignInError> {
    if request.email.is_empty() {
        return Err(SignInError {
            reason: "no email".to_owned(),
            // Cleared on a refusal, which only the error arm can express.
            set_cookie: Some("session=; Max-Age=0".to_owned()),
        });
    }

    Ok(SignedIn {
        user_id: "user_1".to_owned(),
        set_cookie: vec!["session=abc; HttpOnly".to_owned(), "theme=dark".to_owned()],
        request_id: Some("req_1".to_owned()),
    })
}

async fn sign_in_plain(
    _: Arc<()>,
    _: SignInRequest,
    _: reflectapi::Empty,
) -> Result<SignedInPlain, SignInError> {
    Ok(SignedInPlain {
        user_id: "user_1".to_owned(),
    })
}

fn builder() -> reflectapi::Builder<Arc<()>> {
    reflectapi::Builder::new()
        .name("headers")
        .route(sign_in, |b| b.name("auth.sign-in"))
        .route(sign_in_plain, |b| b.name("auth.sign-in-plain"))
}

async fn call(body: &str) -> http::Response<Body> {
    let (_, routers) = builder().build().unwrap();
    let app = reflectapi::axum::into_router(Arc::new(()), routers, |_, r| r);

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/auth.sign-in")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap(),
    )
    .await
    .unwrap()
}

/// The point of the design: a header field is not in the schema, so no client
/// is told to expect a body field it will never receive.
#[test]
fn header_fields_are_absent_from_the_schema() {
    let (schema, _) = builder().build().unwrap();
    let types = serde_json::to_string(&schema).unwrap();

    assert!(types.contains("user_id"));
    assert!(!types.contains("set_cookie"));
    assert!(!types.contains("set-cookie"));
    assert!(!types.contains("x-request-id"));
}

#[tokio::test]
async fn a_vec_field_sends_the_header_once_per_element() {
    let response = call(r#"{"email":"someone@example.com"}"#).await;

    assert_eq!(response.status(), StatusCode::OK);

    let cookies: Vec<&str> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect();
    assert_eq!(cookies, ["session=abc; HttpOnly", "theme=dark"]);
}

#[tokio::test]
async fn a_field_name_becomes_kebab_case_and_a_rename_wins() {
    let response = call(r#"{"email":"someone@example.com"}"#).await;

    // `set_cookie` with no rename.
    assert!(response.headers().contains_key("set-cookie"));
    // `request_id` renamed, so the rename is used verbatim.
    assert_eq!(response.headers().get("x-request-id").unwrap(), "req_1");
}

#[tokio::test]
async fn the_body_carries_only_the_fields_that_are_not_headers() {
    let response = call(r#"{"email":"someone@example.com"}"#).await;
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({ "user_id": "user_1" })
    );
}

/// Unlike a wrapper on the success value, a field works on the error arm too —
/// which is what lets a refusal clear a cookie.
#[tokio::test]
async fn an_error_keeps_its_status_and_can_set_a_header_too() {
    let response = call(r#"{"email":""}"#).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::SET_COOKIE).unwrap(),
        "session=; Max-Age=0"
    );
}

#[tokio::test]
async fn a_none_field_sends_no_header_at_all() {
    let response = call(r#"{"email":""}"#).await;

    assert!(!response.headers().contains_key("x-request-id"));
}

/// A container rule shapes the body, and header fields are not in the body —
/// so `rename_all` must not reach a header name.
#[derive(reflectapi::Output, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CamelCased {
    user_id: String,
    #[reflectapi(header)]
    #[serde(skip_serializing)]
    set_cookie: Option<String>,
    #[reflectapi(header)]
    #[serde(skip_serializing)]
    r#type: Option<String>,
}

#[test]
fn a_container_rename_rule_does_not_reach_the_header_name() {
    use reflectapi::Output;

    let value = CamelCased {
        user_id: "user_1".to_owned(),
        set_cookie: Some("a=1".to_owned()),
        r#type: Some("x".to_owned()),
    };
    let names: Vec<&str> = value
        .reflectapi_response_headers()
        .into_iter()
        .map(|(name, _)| name)
        .collect();

    assert_eq!(names, ["set-cookie", "type"]);
}
