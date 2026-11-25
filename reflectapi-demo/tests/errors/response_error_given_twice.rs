#[derive(serde::Serialize, reflectapi::Output)]
struct Wait {}

impl reflectapi::ResponseError for Wait {
    fn status(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
enum MyError {
    #[reflectapi(status = 404, transparent)]
    NotFound(Wait),
}

fn main() {}
