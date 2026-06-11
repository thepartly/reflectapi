#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct(
    #[reflectapi(hidden)]
    u32,
);

fn main() {}
