#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(deprecated_since = "2026-10-01", sunset = "2026-11-01")]
    field: u32,
}

fn main() {}
