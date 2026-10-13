#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[deprecated]
    #[reflectapi(sunset = "2026-02-30")]
    field: u32,
}

fn main() {}
