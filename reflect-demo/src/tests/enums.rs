#[derive(reflect::Input)]
enum TestEnumEmpty {}
#[test]
fn test_enum_empty() {
    insta::assert_json_snapshot!(TestEnumEmpty::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnum {
    Variant1,
    Variant2,
}
#[test]
fn test_enum() {
    insta::assert_json_snapshot!(TestEnum::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithFields {
    Variant1(u8),
    Variant2(String, f64),
}
#[test]
fn test_enum_with_fields() {
    insta::assert_json_snapshot!(TestEnumWithFields::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithGenerics<T>
where
    T: reflect::Input,
{
    Variant1(T),
    Variant2(T, T),
}
#[test]
fn test_enum_with_generics() {
    insta::assert_json_snapshot!(TestEnumWithGenerics::<u8>::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithGenericsAndFields<T>
where
    T: reflect::Input,
{
    Variant1(u8),
    Variant2(T, T),
}
#[test]
fn test_enum_with_generics_and_fields() {
    insta::assert_json_snapshot!(TestEnumWithGenericsAndFields::<u8>::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithEmptyVariantAndFields {
    Variant1,
    Variant2(u8),
}
#[test]
fn test_enum_with_empty_variant_and_fields() {
    insta::assert_json_snapshot!(TestEnumWithEmptyVariantAndFields::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithBasicVariantAndFieldsAndNamedFields {
    Variant0,
    Variant1(u8, String),
    Variant2 {
        field1: u8,
        field2: String,
    },
}
#[test]
fn test_enum_with_basic_variant_and_fields_and_named_fields() {
    insta::assert_json_snapshot!(TestEnumWithBasicVariantAndFieldsAndNamedFields::reflect_input());
}

#[allow(dead_code)]
#[derive(reflect::Input)]
enum TestEnumWithGenericsAndFieldsAndNamedFields<T>
where
    T: reflect::Input,
{
    Variant1(u8),
    Variant2(T, T),
    Variant3 {
        field1: u8,
        field2: T,
    },
}
#[test]
fn test_enum_with_generics_and_fields_and_named_fields() {
    insta::assert_json_snapshot!(TestEnumWithGenericsAndFieldsAndNamedFields::<u8>::reflect_input());
}

