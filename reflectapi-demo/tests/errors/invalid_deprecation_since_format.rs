#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[deprecated(note = "Use `other` instead.")]
    #[reflectapi(deprecated_since = "2026-1-01")]
    field: u32,
}

fn main() {}
