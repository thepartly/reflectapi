#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[deprecated(since = "2026-13-01")]
    field: u32,
}

fn main() {}
