#[derive(reflect::Input, reflect::Output)]
struct MyStruct<'a> {
    field: &'a u8,
}

fn main() {}
