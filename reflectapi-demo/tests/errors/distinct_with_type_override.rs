#[derive(serde::Deserialize, reflectapi::Input)]
struct Wire(String);

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(from = "Wire")]
#[reflectapi(distinct)]
struct Id(String);

impl From<Wire> for Id {
    fn from(wire: Wire) -> Self {
        Id(wire.0)
    }
}

fn main() {}
