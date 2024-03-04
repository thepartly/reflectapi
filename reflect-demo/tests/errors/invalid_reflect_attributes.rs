#[derive(serde::Deserialize, reflect_derive::Reflect)]
struct MyStruct {
    #[reflect(unknown_attributes)]
    field: u32,
}

fn main() {}
