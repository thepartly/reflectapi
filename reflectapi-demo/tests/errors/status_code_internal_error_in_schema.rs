#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(status = 500)]
    InternalError,
}

fn main() {}
