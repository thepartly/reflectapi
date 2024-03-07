#[derive(reflect::Input, reflect::Output)]
struct MyStruct {
    field: Vec<NotReflectable>,
}

struct NotReflectable;

fn main() {}
