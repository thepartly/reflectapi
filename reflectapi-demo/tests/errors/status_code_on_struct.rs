#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
struct MyError {
    message: String,
}

fn main() {}
