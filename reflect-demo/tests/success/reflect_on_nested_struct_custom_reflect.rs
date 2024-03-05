#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
struct TestStructNested {
    _f: String,
}
impl reflect::Reflect for TestStructNested {
    fn reflect_type(_schema: &mut reflect::Schema) -> String {
        format!("{}::{}", module_path!(), stringify!(TestStructNested))
    }
}

fn main() {}
