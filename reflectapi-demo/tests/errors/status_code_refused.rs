#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(status = 200)]
    Success,
    #[reflectapi(status = 302)]
    Found,
    #[reflectapi(status = 400)]
    BadRequest,
    #[reflectapi(status = 405)]
    MethodNotAllowed,
    #[reflectapi(status = 415)]
    UnsupportedMediaType,
    #[reflectapi(status = 408)]
    RequestTimeout,
    #[reflectapi(status = 413)]
    PayloadTooLarge,
    #[reflectapi(status = 414)]
    UriTooLong,
    #[reflectapi(status = 431)]
    HeadersTooLarge,
    #[reflectapi(status = 503)]
    Unavailable,
    #[reflectapi(status = 600)]
    OutOfRange,
    #[reflectapi(status = 70000)]
    NotANumber,
}

fn main() {}
