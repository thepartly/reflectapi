#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
// #[derive(reflect::Input, reflect::Output)]
struct TestStructNested {
    _f: String,
}

fn main() {}
