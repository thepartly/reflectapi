#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key)]
    tenant: Vec<String>,
}

fn main() {}
