use std::sync::Arc;

pub trait StatusCode {
    fn status_code(&self) -> http::StatusCode;
}

impl<T: StatusCode> StatusCode for Arc<T> {
    fn status_code(&self) -> http::StatusCode {
        (**self).status_code()
    }
}

impl<T: StatusCode> StatusCode for Box<T> {
    fn status_code(&self) -> http::StatusCode {
        (**self).status_code()
    }
}

pub trait IntoResult<O, E> {
    fn into_result(self) -> Result<O, E>;
}

impl<T: crate::Output> IntoResult<T, crate::Infallible> for T {
    fn into_result(self) -> Result<T, crate::Infallible> {
        Result::Ok(self)
    }
}

impl<T, E> IntoResult<T, E> for Result<T, E> {
    fn into_result(self) -> Result<T, E> {
        self
    }
}

/// An output, plus the headers the response should carry.
///
/// A handler returns this in place of its output type when it needs to set a
/// header the body cannot express — a `Set-Cookie`, most often, since a
/// browser will not let script read one back.
///
/// The schema is unaffected: the route still declares the inner type, so
/// generated clients are identical whether or not a route sets headers.
///
/// ```ignore
/// async fn sign_in(state: State, req: SignInRequest, _: Headers)
///     -> Result<reflectapi::WithHeaders<SignedIn>, SignInError>
/// {
///     let session = state.sign_in(req).await?;
///
///     Ok(reflectapi::WithHeaders::new(SignedIn { user_id: session.user_id })
///         .append_header(http::header::SET_COOKIE, session.cookie))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct WithHeaders<T> {
    pub value: T,
    pub headers: http::HeaderMap,
}

impl<T> WithHeaders<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            headers: http::HeaderMap::new(),
        }
    }

    /// Sets a header, replacing any value already held for that name.
    pub fn header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Adds a header, keeping any value already held for that name.
    ///
    /// This is the one to reach for with `Set-Cookie`, where several values
    /// under one name is the normal case rather than a mistake.
    pub fn append_header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.headers.append(name, value);
        self
    }

    pub fn with_headers(mut self, headers: http::HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }
}

impl<T> From<T> for WithHeaders<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
