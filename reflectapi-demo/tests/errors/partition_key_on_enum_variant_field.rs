#[derive(serde::Deserialize, reflectapi::Input)]
enum Request {
    Pay {
        #[reflectapi(partition_key)]
        order: String,
    },
}

fn main() {}
