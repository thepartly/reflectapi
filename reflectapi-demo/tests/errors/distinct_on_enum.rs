#[derive(serde::Deserialize, reflectapi::Input)]
#[reflectapi(distinct)]
enum Shape {
    Circle,
}

fn main() {}
