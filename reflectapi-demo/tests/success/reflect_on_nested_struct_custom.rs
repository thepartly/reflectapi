#[derive(reflectapi::Input, reflectapi::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
struct TestStructNested {
    _f: String,
}
impl reflectapi::Input for TestStructNested {
    fn reflectapi_input_type(_schema: &mut reflectapi::Typespace) -> reflectapi::TypeReference {
        format!("{}::{}", module_path!(), stringify!(TestStructNested)).into()
    }
}
impl reflectapi::Output for TestStructNested {
    fn reflectapi_output_type(_schema: &mut reflectapi::Typespace) -> reflectapi::TypeReference {
        format!("{}::{}", module_path!(), stringify!(TestStructNested)).into()
    }
}

fn main() {}
