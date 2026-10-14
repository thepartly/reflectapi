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

#[test]
fn test_internally_tagged_enum_with_tuple_variants() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    enum A {
        X,
        Y,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum E {
        #[serde(rename = "a")]
        A(A),
    }

    assert_snapshot!(E);
}

#[test]
fn test_internally_tagged_enum_with_optional_fields() {
    // Repro for https://github.com/thepartly/reflectapi/issues/167:
    // a nullable `Option<T>` field on a struct used as an internally-tagged
    // enum variant must be generated as an optional key (serde omits the key
    // when the value is `None`), matching the standalone-struct behavior.
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    pub struct VariantBody {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub amount: Option<String>,
        pub always_null_on_wire: Option<String>,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Thing {
        Variant(VariantBody),
        Named {
            #[serde(skip_serializing_if = "Option::is_none")]
            amount: Option<String>,
        },
    }

    assert_snapshot!(Thing);
}

#[test]
fn test_struct_with_flattened_internally_tagged_enum_with_optional_fields() {
    // Repro for https://github.com/thepartly/reflectapi/issues/167:
    // same as test_internally_tagged_enum_with_optional_fields, but with the
    // enum flattened into a parent struct, which is rendered by a separate
    // code path generating one model per variant.
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    pub struct VariantBody {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub amount: Option<String>,
        pub always_null_on_wire: Option<String>,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Thing {
        Variant(VariantBody),
        Named {
            #[serde(skip_serializing_if = "Option::is_none")]
            amount: Option<String>,
        },
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    pub struct Offer {
        pub id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<String>,
        #[serde(flatten)]
        pub details: Thing,
    }

    assert_snapshot!(Offer);
}

#[test]
fn test_internally_tagged_enum_with_unit_variants() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum A {
        X,
        Y(()),
    }

    assert_snapshot!(A);
}

#[test]
fn test_internally_tagged_enum_wrapping_untagged_enum() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Evaluation {
        score: u32,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Failure {
        reason: String,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(untagged)]
    enum Outcome {
        Evaluation(Evaluation),
        Failure(Failure),
    }

    // Each variant's branch is a `oneOf` of the inner enum's variants; the two differ only in
    // their tag, so only a title lets a diff tool tell them apart.
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum Rule {
        First(Outcome),
        Second(Outcome),
    }

    assert_snapshot!(Rule);
}

#[test]
fn test_internally_tagged_variants_wrapping_one_struct() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Body {
        x: u32,
    }

    // Both branches would otherwise carry `Body`'s name as their title.
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum E {
        #[serde(rename = "first")]
        A(Body),
        #[serde(rename = "second")]
        C(Body),
    }

    assert_snapshot!(E);
}
