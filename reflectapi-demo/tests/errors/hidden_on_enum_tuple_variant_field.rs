#[derive(serde::Deserialize, reflectapi::Input)]
enum MyEnum {
    Variant(
        #[reflectapi(hidden)]
        u32,
    ),
}

fn main() {}
