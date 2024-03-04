#[derive(reflect::Input)]
struct MyStruct {
    field: fn(&str) -> i32,
}

fn main() {}
