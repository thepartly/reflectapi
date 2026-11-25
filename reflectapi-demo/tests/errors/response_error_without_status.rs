#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
enum MyError {
    #[reflectapi(status = 404)]
    NotFound,
    Conflict,
}

fn main() {}
