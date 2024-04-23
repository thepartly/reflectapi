#[derive(reflectapi::Input)]
struct MyStruct {
    field: dyn Fn(&str) -> i32,
}

fn main() {}
