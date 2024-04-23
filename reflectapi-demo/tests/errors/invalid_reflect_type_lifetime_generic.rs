#[derive(reflectapi::Input, reflectapi::Output)]
struct MyStruct<'a> {
    field: &'a u8,
}

fn main() {}
