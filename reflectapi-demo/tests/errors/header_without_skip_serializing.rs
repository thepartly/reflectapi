#[derive(serde::Serialize, reflectapi::Output)]
struct Response {
    user_id: String,
    // Without `#[serde(skip_serializing)]` this value would be sent in the
    // body as well as the header.
    #[reflectapi(header)]
    set_cookie: String,
}

fn main() {}
