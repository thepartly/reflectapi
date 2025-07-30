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

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct CarData {
    vin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mileage: Option<u32>,
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum Vehicle {
    #[serde(rename = "car")]
    Car(CarData), // Tuple variant to flatten

    #[serde(rename = "truck")]
    Truck { license_plate: String },

    #[serde(rename = "bicycle")]
    Bicycle, // Unit variant
}

#[test]
fn test_internally_tagged_enum_with_tuple_variants() {
    assert_snapshot!(Vehicle);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct ImageData {
    width: u32,
    height: u32,
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct VideoData {
    duration: u32,
    resolution: Option<String>,
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "media_type")]
enum Media {
    #[serde(rename = "image")]
    Image(ImageData), // Tuple variant

    #[serde(rename = "video")]
    Video(VideoData), // Another tuple variant

    #[serde(rename = "audio")]
    Audio { url: String, format: Option<String> }, // Struct variant

    #[serde(rename = "text")]
    Text, // Unit variant
}

#[test]
fn test_internally_tagged_enum_mixed_variants() {
    assert_snapshot!(Media);
}

// Test tuple variants with user-defined enum types
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct ProductInfo {
    name: String,
    price: f64,
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "item_type")]
enum ShopItem {
    #[serde(rename = "product")]
    Product(ProductInfo), // Tuple variant with user-defined struct

    #[serde(rename = "colored_item")]
    ColoredItem(Color), // Tuple variant with user-defined enum

    #[serde(rename = "bundle")]
    Bundle { items: Vec<String>, discount: f64 }, // Struct variant

    #[serde(rename = "gift_card")]
    GiftCard, // Unit variant
}

#[test]
fn test_internally_tagged_enum_with_user_defined_types() {
    assert_snapshot!(ShopItem);
}

// Test tuple variants with boxed types (common pattern for recursive structures)
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct BoxedData {
    id: u32,
    description: String,
}

#[allow(dead_code)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "content_type")]
enum BoxedContent {
    #[serde(rename = "boxed")]
    Boxed(Box<BoxedData>), // Tuple variant with boxed struct

    #[serde(rename = "raw")]
    Raw { content: String }, // Struct variant

    #[serde(rename = "empty")]
    Empty, // Unit variant
}

#[test]
fn test_internally_tagged_enum_with_boxed_types() {
    assert_snapshot!(BoxedContent);
}
