#[derive(serde::Serialize, reflectapi::Output)]
struct Wait {}

impl reflectapi::StatusCode for Wait {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(status = 404, transparent)]
    NotFound(Wait),
}

fn main() {}
