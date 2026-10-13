#[derive(serde::Deserialize, reflectapi::Input)]
enum MyEnum {
    #[deprecated]
    #[reflectapi(deprecated_since = "2026-11-01", sunset = "2026-10-01")]
    Variant,
}

fn main() {}
