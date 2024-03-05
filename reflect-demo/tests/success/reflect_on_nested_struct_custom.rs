#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
struct TestStructNested {
    _f: String,
}
impl reflect::Input for TestStructNested {
    fn reflect_input() -> reflect::Schema {
        reflect::Schema::new()
    }
    fn reflect_input_name() -> String {
        format!("{}::{}", module_path!(), stringify!(TestStructNested))
    }
    fn reflect_input_type(_schema: &mut reflect::Schema) -> () {}
}
impl reflect::Output for TestStructNested {
    fn reflect_output() -> reflect::Schema {
        reflect::Schema::new()
    }
    fn reflect_output_name() -> String {
        format!("{}::{}", module_path!(), stringify!(TestStructNested))
    }
    fn reflect_output_type(_schema: &mut reflect::Schema) -> () {}
}

fn main() {}
