#[derive(reflect::Input, reflect::Output)]
struct MyStruct<'a> {
    field: std::borrow::Cow<'a, u8>,
}

fn main() {}
