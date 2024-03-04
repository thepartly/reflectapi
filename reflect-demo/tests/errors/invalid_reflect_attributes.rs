#[derive(serde::Deserialize, reflect::Reflect)]
struct MyStruct {
    #[reflect(unknown_attributes)]
    field: u32,
}

fn main() {}
