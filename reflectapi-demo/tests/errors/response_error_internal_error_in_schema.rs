#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
enum MyError {
    #[reflectapi(status = 500)]
    InternalError,
}

fn main() {}
