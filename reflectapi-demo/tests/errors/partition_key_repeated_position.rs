#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key = 1)]
    tenant: String,
    #[reflectapi(partition_key = 1)]
    order: String,
}

fn main() {}
