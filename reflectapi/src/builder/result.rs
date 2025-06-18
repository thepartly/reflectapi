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
