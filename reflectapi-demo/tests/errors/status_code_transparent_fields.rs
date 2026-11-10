#[derive(serde::Serialize, reflectapi::Output)]
struct Wait {}

impl reflectapi::StatusCode for Wait {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(transparent)]
    Nothing,
    #[reflectapi(transparent)]
    Two(Wait, Wait),
    #[reflectapi(output_skip, transparent)]
    InternalError(Wait),
}

fn main() {}
