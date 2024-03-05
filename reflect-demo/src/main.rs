mod test_lib;

// #[derive(reflect::Input, reflect::Output)]
// struct MyStruct {
//     /// some docs
//     // #[serde(flatten)]
//     // #[reflect(invalid)]
//     _f: u32,
//     _f2: i8,
// }

// #[derive(reflect::Input, reflect::Output)]
// struct TestStructWithNested {
//     _f: TestStructNested,
// }
// #[derive(reflect::Input, reflect::Output)]
// struct TestStructNested {
//     _f: String,
// }

#[derive(reflect::Input)]
struct TestStructWithVec {
    // _f: Vec<u8>,
}

#[derive(reflect::Input)]
struct TestStructWithNestedExternal {
    _f: crate::test_lib::TestStructNested,
}

trait MyTrait {}

// impl<T: serde::Serialize> MyTrait for T {}
// impl<'de, T: serde::Deserialize<'de>> MyTrait for T {}

fn main() {
    // println!("{}", MyStruct::reflect_input());
    println!("{}", TestStructWithNestedExternal::reflect_input());
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
        insta::assert_snapshot!(TestStructOneBasicFieldU32::reflect_input());
    }

    #[derive(reflect::Input)]
    struct TestStructOneBasicFieldString {
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string() {
        insta::assert_snapshot!(TestStructOneBasicFieldString::reflect_input());
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBoth {
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_input() {
        insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_input());
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_output() {
        insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_output());
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBothDifferently {
        #[reflect(output_type = "u32", input_type = "i32")]
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_with_attributes_input() {
        insta::assert_snapshot!(
            TestStructOneBasicFieldStringReflectBothDifferently::reflect_input()
        );
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_with_attributes_output() {
        insta::assert_snapshot!(
            TestStructOneBasicFieldStringReflectBothDifferently::reflect_output()
        );
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBothEqually {
        #[reflect(output_type = "u32", input_type = "u32")]
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_equally_input() {
        insta::allow_duplicates! {
            insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_input());
        }
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_equally_output() {
        insta::allow_duplicates! {
            insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_output());
        }
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructOneBasicFieldStringReflectBothEqually2 {
        #[reflect(type = "u32")]
        _f: String,
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_equally2_input() {
        insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_input());
    }
    #[test]
    fn test_reflect_struct_one_basic_field_string_reflect_both_equally2_output() {
        insta::assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually2::reflect_output());
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructWithNested {
        _f: TestStructNested,
    }
    #[derive(reflect::Input, reflect::Output)]
    struct TestStructNested {
        _f: String,
    }
    #[test]
    fn test_reflect_struct_with_nested_input() {
        insta::assert_snapshot!(TestStructWithNested::reflect_input());
    }
    #[test]
    fn test_reflect_struct_with_nested_output() {
        insta::assert_snapshot!(TestStructWithNested::reflect_output());
    }

    #[derive(reflect::Input, reflect::Output)]
    struct TestStructWithNestedExternal {
        _f: crate::test_lib::TestStructNested,
    }
    #[test]
    fn test_reflect_struct_with_nested_external_input() {
        insta::assert_snapshot!(TestStructWithNestedExternal::reflect_input());
    }
    #[test]
    fn test_reflect_struct_with_nested_external_output() {
        insta::assert_snapshot!(TestStructWithNestedExternal::reflect_output());
    }
}
