pub trait StatusCode {
    fn status_code(&self) -> u16;
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum Result<T, E>
where
    T: crate::Output,
    E: crate::Output + StatusCode,
{
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E>
where
    T: crate::Output,
    E: crate::Output + StatusCode,
{
    pub fn status_code(&self) -> u16 {
        match self {
            Result::Ok(_) => 200,
            Result::Err(e) => {
                let custom_error = e.status_code();
                if custom_error == 200 {
                    // It means a user has implemented ToStatusCode trait for their
                    // type incorrectly. It is a protocol error to return 200 status
                    // code for an error response, as the client will not be able
                    // to "cast" the response body to the correct type.
                    // So, we are reverting it to internal error
                    500
                } else {
                    custom_error
                }
            }
        }
    }
}

impl<T, E> From<std::result::Result<T, E>> for Result<T, E>
where
    T: crate::Output,
    E: crate::Output + StatusCode,
{
    fn from(r: std::result::Result<T, E>) -> Self {
        match r {
            Ok(t) => Result::Ok(t),
            Err(e) => Result::Err(e),
        }
    }
}

impl<T> From<T> for Result<T, crate::Infallible>
where
    T: crate::Output,
{
    fn from(t: T) -> Self {
        Result::Ok(t)
    }
}
