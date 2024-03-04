use reflect::Reflect;

#[derive(Reflect)]
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
    // let inst = MyStruct { _f: 42 };
    // println!("{}", MyStruct::reflect());
    // println!("{}", inst.reflect_debug());
    println!("{:#?}", MyStruct::reflect());
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

    #[derive(reflect::Reflect)]
    struct TestStructOneBasicField {
        _f: u32,
    }
    #[test]
    fn test_reflect_struct_one_basic_field() {
        insta::assert_debug_snapshot!(TestStructOneBasicField::reflect());
    }
}
