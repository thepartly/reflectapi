#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key)]
    #[serde(skip)]
    tenant: String,
}

fn main() {}
