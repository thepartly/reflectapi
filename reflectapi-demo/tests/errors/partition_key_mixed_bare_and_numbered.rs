#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key)]
    tenant: String,
    #[reflectapi(partition_key = 2)]
    order: String,
}

fn main() {}
