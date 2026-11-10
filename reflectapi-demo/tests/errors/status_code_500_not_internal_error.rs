#[derive(serde::Serialize, reflectapi::Output, reflectapi::StatusCode)]
enum MyError {
    #[reflectapi(output_skip, status = 500)]
    Failed,
}

fn main() {}
