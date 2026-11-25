#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
struct MyError {
    message: String,
}

fn main() {}
