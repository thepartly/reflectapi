#[derive(serde::Deserialize, reflect::Input)]
struct MyStruct {
    #[reflect(unknown_attributes)]
    field: u32,
}

fn main() {}
