// Both derives validate the dates; the error must be reported once.
#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
enum MyEnum {
    #[deprecated]
    #[reflectapi(deprecated_since = "2026-11-01", sunset = "2026-10-01")]
    Variant,
}

fn main() {}
