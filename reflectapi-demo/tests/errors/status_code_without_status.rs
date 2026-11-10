#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(status = 404)]
    NotFound,
    Conflict,
}

fn main() {}
