#[derive(serde::Deserialize, reflectapi::Input)]
#[reflectapi(distinct)]
struct Point {
    x: u32,
    y: u32,
}

fn main() {}
