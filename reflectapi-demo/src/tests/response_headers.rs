//! `route_with_headers`: a handler that sets response headers without the
//! schema — and so every generated client — changing at all.

use std::sync::Arc;

use axum::body::Body;
use http::{header, Request, StatusCode};
use tower::ServiceExt;

#[derive(reflectapi::Input, serde::Deserialize)]
struct SignInRequest {
    email: String,
}

#[derive(reflectapi::Output, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct SignedIn {
    user_id: String,
}

#[derive(reflectapi::Output, serde::Serialize)]
struct SignInError {
    reason: String,
}

impl reflectapi::StatusCode for SignInError {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

async fn sign_in(
    _: Arc<()>,
    request: SignInRequest,
    _: reflectapi::Empty,
) -> Result<reflectapi::WithHeaders<SignedIn>, SignInError> {
    if request.email.is_empty() {
        return Err(SignInError {
            reason: "no email".to_owned(),
        });
    }

    Ok(reflectapi::WithHeaders::new(SignedIn {
        user_id: "user_1".to_owned(),
    })
    .append_header(header::SET_COOKIE, "session=abc; HttpOnly".parse().unwrap())
    .append_header(header::SET_COOKIE, "theme=dark".parse().unwrap()))
}

async fn sign_in_plain(
    _: Arc<()>,
    _: SignInRequest,
    _: reflectapi::Empty,
) -> Result<SignedIn, SignInError> {
    Ok(SignedIn {
        user_id: "user_1".to_owned(),
    })
}

fn builder() -> reflectapi::Builder<Arc<()>> {
    reflectapi::Builder::new()
        .name("headers")
        .route_with_headers(sign_in, |b| b.name("auth.sign-in"))
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

/// The point of the whole design: a route that sets headers is
/// indistinguishable in the schema from one that does not, so no client
/// changes shape because a server started setting a cookie.
#[test]
fn setting_headers_does_not_change_the_schema() {
    let (schema, _) = builder().build().unwrap();

    let with = schema
        .functions
        .iter()
        .find(|f| f.name == "auth.sign-in")
        .unwrap();
    let without = schema
        .functions
        .iter()
        .find(|f| f.name == "auth.sign-in-plain")
        .unwrap();

    let shape = |function: &reflectapi::Function| {
        let mut value = serde_json::to_value(function).unwrap();
        value.as_object_mut().unwrap().remove("name");
        value
    };

    assert_eq!(shape(with), shape(without));
}

#[tokio::test]
async fn the_headers_reach_the_response() {
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
async fn the_body_is_the_declared_type_and_nothing_else() {
    let response = call(r#"{"email":"someone@example.com"}"#).await;
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    assert_eq!(
        serde_json::from_slice::<SignedIn>(&body).unwrap(),
        SignedIn {
            user_id: "user_1".to_owned()
        }
    );
}

#[tokio::test]
async fn content_type_is_still_answered_in_the_request_format() {
    let response = call(r#"{"email":"someone@example.com"}"#).await;

    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
}

/// Errors keep their declared status and carry no headers, because only the
/// success arm has anywhere to put them.
#[tokio::test]
async fn an_error_keeps_its_status_and_sets_no_cookie() {
    let response = call(r#"{"email":""}"#).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}
