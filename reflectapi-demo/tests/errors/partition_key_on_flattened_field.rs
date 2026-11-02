#[derive(serde::Deserialize, reflectapi::Input)]
struct Order {
    id: String,
}

#[derive(serde::Deserialize, reflectapi::Input)]
struct Request {
    #[reflectapi(partition_key)]
    #[serde(flatten)]
    order: Order,
}

fn main() {}
