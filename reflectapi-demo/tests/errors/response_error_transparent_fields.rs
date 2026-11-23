#[derive(serde::Serialize, reflectapi::Output)]
struct Wait {}

impl reflectapi::ResponseError for Wait {
    fn status(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
enum MyError {
    #[reflectapi(transparent)]
    Nothing,
    #[reflectapi(transparent)]
    Two(Wait, Wait),
    #[reflectapi(output_skip, transparent)]
    InternalError(Wait),
}

fn main() {}
