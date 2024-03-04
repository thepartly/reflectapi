#[derive(reflect::Input)]
struct MyStruct {
    /// some docs
    // #[serde(flatten)]
    // #[reflect(invalid)]
    _f: u32,
}

trait MyTrait {}

// impl<T: serde::Serialize> MyTrait for T {}
impl<'de, T: serde::Deserialize<'de>> MyTrait for T {}

fn main() {
    println!("{:#?}", MyStruct::reflect_input());
}

#[cfg(test)]
mod test {
    #[test]
    fn compiler_error_cases() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/errors/*.rs");
    }

    #[test]
    fn compiler_success_cases() {
        let t = trybuild::TestCases::new();
        t.pass("tests/success/*.rs");
    }

    #[derive(reflect::Input)]
    struct TestStructOneBasicFieldU32 {
        _f: u32,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_u32() {
        insta::assert_debug_snapshot!(TestStructOneBasicFieldU32::reflect_input());
    }

    #[derive(reflect::Input)]
    struct TestStructOneBasicFieldString {
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string() {
        insta::assert_debug_snapshot!(TestStructOneBasicFieldString::reflect_input());
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBoth {
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both() {
        insta::allow_duplicates! {
            insta::assert_debug_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_input());
            insta::assert_debug_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_output());
        }
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBothDifferently {
        #[reflect(output_type = "u32", input_type = "i32")]
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_input() {
        insta::allow_duplicates! {
            insta::assert_debug_snapshot!(TestStructOneBasicFieldStringReflectBothDifferently::reflect_input());
        }
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_output() {
        insta::allow_duplicates! {
            insta::assert_debug_snapshot!(TestStructOneBasicFieldStringReflectBothDifferently::reflect_output());
        }
    }
}
