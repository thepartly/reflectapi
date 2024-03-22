#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
struct TestStructNested {
    _f: String,
}
impl reflect::Input for TestStructNested {
    fn reflect_input_type(_schema: &mut reflect::Typespace) -> reflect::TypeReference {
        format!("{}::{}", module_path!(), stringify!(TestStructNested)).into()
    }
}
impl reflect::Output for TestStructNested {
    fn reflect_output_type(_schema: &mut reflect::Typespace) -> reflect::TypeReference {
        format!("{}::{}", module_path!(), stringify!(TestStructNested)).into()
    }
}

fn main() {}
