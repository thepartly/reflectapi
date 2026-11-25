use std::sync::Arc;

/// How a route's error is answered: the status of its response, and the
/// headers the response adds. A route's error is a type the server writes,
/// so it is also [`Output`](crate::Output) and `Serialize`.
///
/// Derive it on a route's error enum, `#[derive(reflectapi::ResponseError)]`,
/// rather than implement it: the derive checks every status as it compiles,
/// and a hand-written impl isn't checked. Implement it by hand only for a type
/// a `#[reflectapi(transparent)]` variant wraps; one with only a status
/// implements `status` alone.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a route's error: it doesn't implement `reflectapi::ResponseError`",
    note = "derive it on an error enum: `#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]`",
    note = "see https://reflectapi.rs/getting-started/attributes.html#a-routes-error"
)]
pub trait ResponseError: crate::Output + serde::Serialize {
    /// The status of the response.
    fn status(&self) -> http::StatusCode;

    /// Headers to add to the response, such as `retry-after` or
    /// `www-authenticate`; none by default.
    fn headers(&self) -> http::HeaderMap {
        http::HeaderMap::new()
    }
}

/// A [`ResponseError`] with only a status: implementing it implements
/// `ResponseError`, with no headers.
#[deprecated(
    since = "0.27.0",
    note = "implement `reflectapi::ResponseError` instead, renaming `status_code` to `status`, \
            or derive it on an error enum"
)]
pub trait StatusCode {
    fn status_code(&self) -> http::StatusCode;
}

#[allow(deprecated)]
impl<T: StatusCode + crate::Output + serde::Serialize> ResponseError for T {
    fn status(&self) -> http::StatusCode {
        self.status_code()
    }
}

impl<T: ResponseError> ResponseError for Arc<T>
where
    Arc<T>: serde::Serialize,
{
    fn status(&self) -> http::StatusCode {
        (**self).status()
    }

    fn headers(&self) -> http::HeaderMap {
        (**self).headers()
    }
}

// `Box` forwards only the deprecated `StatusCode`: a `ResponseError` impl for
// `Box<T>` would overlap the blanket impl above, as `Box` is `#[fundamental]`
// and a crate may implement `StatusCode` for `Box<ItsType>`. Once
// `StatusCode` is removed, `Box<T>` forwards `ResponseError` as `Arc<T>` does.
// Until then an app boxing a derived error implements `ResponseError` for
// `Box<ItsError>` itself.
#[allow(deprecated)]
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
