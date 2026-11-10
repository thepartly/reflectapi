//! How a route's error is answered: `StatusCode::response_headers` and the
//! `StatusCode` derive.

use reflectapi::dispatch::{Dispatcher, Reply};
use reflectapi::{DispatchOnly, StatusCode as _, WithSchema};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Answered `401`, asking the client to authenticate.
#[derive(Debug, serde::Serialize, reflectapi::Output)]
pub struct Unauthorized {}

impl reflectapi::StatusCode for Unauthorized {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::UNAUTHORIZED
    }

    fn response_headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        headers.append("www-authenticate", http::HeaderValue::from_static("Basic"));
        headers.append("www-authenticate", http::HeaderValue::from_static("Bearer"));
        // reflectapi's own: the response keeps its content type.
        headers.insert("content-type", http::HeaderValue::from_static("text/html"));
        headers
    }
}

/// An app's own "try again after": `429`, with the wait in `retry-after`.
#[derive(Debug, serde::Serialize, reflectapi::Output)]
pub struct RetryAfter {
    pub after_secs: u64,
}

impl reflectapi::StatusCode for RetryAfter {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }

    fn response_headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        headers.insert("retry-after", http::HeaderValue::from(self.after_secs));
        headers
    }
}

#[derive(Debug, serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
pub enum PetsInsertError {
    /// Its status and headers are the field's.
    #[reflectapi(transparent)]
    Unauthorized(Unauthorized),
    #[reflectapi(transparent)]
    TryAgain(RetryAfter),
    #[reflectapi(status = 404)]
    OwnerNotFound,
    #[reflectapi(status = 409)]
    Conflict { name: String },
    #[reflectapi(status = 422)]
    NameInvalid(String),
    #[reflectapi(output_skip, status = 500)]
    InternalError,
}

/// A generic error: the derive bounds its field's type by `StatusCode`.
#[derive(Debug, serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
pub enum Wrapped<E> {
    #[reflectapi(transparent)]
    Inner(E),
    #[reflectapi(status = 410)]
    Gone,
}

#[test]
fn the_derive_gives_each_variant_its_status() {
    let statuses = [
        (PetsInsertError::Unauthorized(Unauthorized {}), 401),
        (
            PetsInsertError::TryAgain(RetryAfter { after_secs: 30 }),
            429,
        ),
        (PetsInsertError::OwnerNotFound, 404),
        (
            PetsInsertError::Conflict {
                name: "Tweety".into(),
            },
            409,
        ),
        (PetsInsertError::NameInvalid("".into()), 422),
        (PetsInsertError::InternalError, 500),
    ];
    for (error, status) in statuses {
        assert_eq!(error.status_code().as_u16(), status, "{error:?}");
    }
    assert_eq!(
        Wrapped::Inner(PetsInsertError::OwnerNotFound)
            .status_code()
            .as_u16(),
        404
    );
    assert_eq!(Wrapped::<PetsInsertError>::Gone.status_code().as_u16(), 410);
}

#[test]
fn the_derive_passes_a_transparent_fields_headers_on() {
    let headers = PetsInsertError::TryAgain(RetryAfter { after_secs: 30 }).response_headers();
    assert_eq!(
        headers.get("retry-after").map(|v| v.as_bytes()),
        Some(&b"30"[..])
    );
    assert!(PetsInsertError::OwnerNotFound.response_headers().is_empty());
    assert_eq!(
        Wrapped::Inner(PetsInsertError::Unauthorized(Unauthorized {}))
            .response_headers()
            .get_all("www-authenticate")
            .iter()
            .count(),
        2
    );
}

mod pets {
    use super::{PetsInsertError, RetryAfter, Unauthorized};

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsInsertRequest {
        pub answer: String,
    }

    pub async fn pets_insert(
        _: (),
        request: PetsInsertRequest,
        _: reflectapi::Empty,
    ) -> Result<reflectapi::Empty, PetsInsertError> {
        Err(match request.answer.as_str() {
            "unauthorized" => PetsInsertError::Unauthorized(Unauthorized {}),
            "try-again" => PetsInsertError::TryAgain(RetryAfter { after_secs: 2 }),
            _ => PetsInsertError::OwnerNotFound,
        })
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().route(pets_insert, |b| b.name("pets.insert"))
    }
}

async fn answer(
    answer: &str,
) -> Result<(http::StatusCode, http::HeaderMap, bytes::Bytes), Box<dyn std::error::Error>> {
    let dispatcher = Dispatcher::new(pets::api::<DispatchOnly>().build()?);
    let mut headers = http::HeaderMap::new();
    headers.insert(
        "content-type",
        http::HeaderValue::from_static("application/json"),
    );
    let reply = dispatcher
        .dispatch(
            (),
            &http::Method::POST,
            "/pets.insert",
            &headers,
            bytes::Bytes::from(format!(r#"{{"answer":"{answer}"}}"#)),
        )
        .await
        .ok_or("no route")?;
    let Reply::Complete {
        status,
        headers,
        body,
    } = reply
    else {
        return Err(format!("expected a response, got {reply:?}").into());
    };
    Ok((status, headers, body))
}

#[tokio::test]
async fn an_errors_response_carries_its_headers() -> TestResult {
    let (status, headers, body) = answer("try-again").await?;
    assert_eq!(status, http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        headers.get("retry-after").map(|v| v.as_bytes()),
        Some(&b"2"[..])
    );
    assert_eq!(body, r#"{"TryAgain":{"after_secs":2}}"#);

    let (status, headers, _) = answer("unauthorized").await?;
    assert_eq!(status, http::StatusCode::UNAUTHORIZED);
    let challenges = headers
        .get_all("www-authenticate")
        .iter()
        .map(|value| value.as_bytes())
        .collect::<Vec<_>>();
    assert_eq!(challenges, [&b"Basic"[..], &b"Bearer"[..]]);
    // The error names `content-type`, which reflectapi sets: reflectapi's stays.
    let content_types = headers.get_all("content-type").iter().collect::<Vec<_>>();
    assert_eq!(
        content_types,
        [&http::HeaderValue::from_static("application/json")]
    );

    let (status, headers, _) = answer("owner-not-found").await?;
    assert_eq!(status, http::StatusCode::NOT_FOUND);
    assert!(headers.get("retry-after").is_none());
    Ok(())
}

#[tokio::test]
async fn an_errors_response_carries_its_headers_through_axum() -> TestResult {
    use tower::Service as _;

    let (_schema, routers) = pets::api::<WithSchema>().build()?;
    let mut app = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    let response = app
        .call(
            http::Request::builder()
                .method(http::Method::POST)
                .uri("/pets.insert")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#"{"answer":"try-again"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response.headers().get("retry-after").map(|v| v.as_bytes()),
        Some(&b"2"[..])
    );
    Ok(())
}

#[test]
fn the_schema_leaves_internal_error_out() -> TestResult {
    let (schema, _) = pets::api::<WithSchema>().build()?;
    let error = schema
        .output_types
        .get_type("status_code::PetsInsertError")
        .ok_or("no error type")?;
    let reflectapi::Type::Enum(error) = error else {
        return Err(format!("expected an enum, got {error:?}").into());
    };
    let variants = error.variants().map(|v| v.name()).collect::<Vec<_>>();
    assert_eq!(
        variants,
        [
            "Unauthorized",
            "TryAgain",
            "OwnerNotFound",
            "Conflict",
            "NameInvalid"
        ]
    );
    Ok(())
}
