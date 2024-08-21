pub trait StatusCode {
    fn status_code(&self) -> http::StatusCode;
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
