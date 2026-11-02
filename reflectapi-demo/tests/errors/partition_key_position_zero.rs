#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key = 0)]
    tenant: String,
}

fn main() {}
