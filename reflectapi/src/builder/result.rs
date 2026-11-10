use std::sync::Arc;

/// How a route's error is answered: its status, and the headers its response
/// carries.
pub trait StatusCode {
    fn status_code(&self) -> http::StatusCode;

    /// Headers to add to this error's response, such as `retry-after` or
    /// `www-authenticate`; none by default.
    fn response_headers(&self) -> http::HeaderMap {
        http::HeaderMap::new()
    }
}

impl<T: StatusCode> StatusCode for Arc<T> {
    fn status_code(&self) -> http::StatusCode {
        (**self).status_code()
    }

    fn response_headers(&self) -> http::HeaderMap {
        (**self).response_headers()
    }
}

impl<T: StatusCode> StatusCode for Box<T> {
    fn status_code(&self) -> http::StatusCode {
        (**self).status_code()
    }

    fn response_headers(&self) -> http::HeaderMap {
        (**self).response_headers()
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
