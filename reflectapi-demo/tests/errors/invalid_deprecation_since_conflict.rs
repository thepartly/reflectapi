#[derive(serde::Deserialize, reflectapi::Input)]
enum MyEnum {
    #[deprecated]
    #[reflectapi(deprecated_since = "2026-10-01")]
    #[reflectapi(deprecated_since = "2026-10-02")]
    Variant,
}

fn main() {}
