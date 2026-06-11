#[derive(serde::Deserialize, reflectapi::Input)]
struct MyNewtype(
    #[reflectapi(hidden)]
    String,
);

fn main() {}
