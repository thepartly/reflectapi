#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumEmpty {}
#[test]
fn test_enum_empty() {
    assert_input_snapshot!(TestEnumEmpty);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnum {
    Variant1,
    Variant2,
}
#[test]
fn test_enum() {
    assert_input_snapshot!(TestEnum);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithFields {
    Variant1(u8),
    Variant2(String, f64),
}
#[test]
fn test_enum_with_fields() {
    assert_input_snapshot!(TestEnumWithFields);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithGenerics<T>
where
    T: reflectapi::Input,
{
    Variant1(T),
    Variant2(T, T),
}
#[test]
fn test_enum_with_generics() {
    assert_input_snapshot!(TestEnumWithGenerics::<u8>);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithGenericsAndFields<T>
where
    T: reflectapi::Input,
{
    Variant1(u8),
    Variant2(T, T),
}
#[test]
fn test_enum_with_generics_and_fields() {
    assert_input_snapshot!(TestEnumWithGenericsAndFields::<u8>);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithEmptyVariantAndFields {
    Variant1,
    Variant2(u8),
}
#[test]
fn test_enum_with_empty_variant_and_fields() {
    assert_input_snapshot!(TestEnumWithEmptyVariantAndFields);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithBasicVariantAndFieldsAndNamedFields {
    Variant0,
    Variant1(u8, String),
    Variant2 { field1: u8, field2: String },
}
#[test]
fn test_enum_with_basic_variant_and_fields_and_named_fields() {
    assert_input_snapshot!(TestEnumWithBasicVariantAndFieldsAndNamedFields);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumWithGenericsAndFieldsAndNamedFields<T>
where
    T: reflectapi::Input,
{
    Variant1(u8),
    Variant2(T, T),
    Variant3 { field1: u8, field2: T },
}
#[test]
fn test_enum_with_generics_and_fields_and_named_fields() {
    assert_input_snapshot!(TestEnumWithGenericsAndFieldsAndNamedFields::<u8>);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[reflectapi(discriminant)]
enum TestEnumWithDiscriminant {
    Variant1 = 1,
    Variant2 = 2,
}
#[test]
fn test_enum_with_discriminant_input() {
    assert_input_snapshot!(TestEnumWithDiscriminant);
}
#[test]
fn test_enum_with_discriminant_output() {
    assert_output_snapshot!(TestEnumWithDiscriminant);
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithDiscriminantIgnored {
    Variant1 = 1,
    Variant2 = 2,
}
#[test]
fn test_enum_with_discriminant_ignored_input() {
    assert_input_snapshot!(TestEnumWithDiscriminantIgnored);
}
#[test]
fn test_enum_with_discriminant_ignored_output() {
    assert_output_snapshot!(TestEnumWithDiscriminantIgnored);
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
pub enum Nums {
    #[serde(rename = "0")]
    Zero = 0,
    A = 1,
}

#[test]
fn test_enum_rename_num() {
    assert_snapshot!(Nums);
}
