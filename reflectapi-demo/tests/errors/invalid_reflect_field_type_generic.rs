#[derive(reflectapi::Input, reflectapi::Output)]
struct MyStruct {
    field: Vec<NotReflectable>,
}

struct NotReflectable;

fn main() {}
