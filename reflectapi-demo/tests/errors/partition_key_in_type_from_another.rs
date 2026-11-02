#[derive(serde::Deserialize, reflectapi::Input)]
struct Wire {
    tenant: String,
}

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(from = "Wire")]
struct Request {
    #[reflectapi(partition_key)]
    tenant: String,
}

impl From<Wire> for Request {
    fn from(wire: Wire) -> Self {
        Request {
            tenant: wire.tenant,
        }
    }
}

fn main() {}
