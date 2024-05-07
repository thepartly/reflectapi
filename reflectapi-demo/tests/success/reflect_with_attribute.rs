#[derive(reflectapi::Input)]
struct MyStruct {
    #[reflectapi(type = "u32")]
    field: u32,
}

fn main() {}
