#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MyStruct")]
struct TestStructRename {}
#[test]
fn test_struct_rename() {
    assert_snapshot!(TestStructRename);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename(serialize = "MyStructOutput", deserialize = "MyStructInput"))]
struct TestStructRenameDifferently {}
#[test]
fn test_struct_rename_differently() {
    assert_snapshot!(TestStructRenameDifferently);
}

#[test]
fn test_kebab_case() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "kebab-case")]
    struct Test {
        field_name: u8,
    }

    assert_snapshot!(Test);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TestStructRenameAll {
    field_name: u8,
}
#[test]
fn test_struct_rename_all() {
    assert_snapshot!(TestStructRenameAll);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TestStructRenameAllDifferently {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_differently() {
    assert_snapshot!(TestStructRenameAllDifferently);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestStructRenameAllPascalCase {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_pascal_case() {
    assert_snapshot!(TestStructRenameAllPascalCase);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructRenameField {
    #[serde(rename(serialize = "field_name", deserialize = "fieldName"))]
    f: u8,
}
#[test]
fn test_struct_rename_field() {
    assert_snapshot!(TestStructRenameField);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MyEnum")]
enum TestEnumRename {
    #[serde(rename = "V1")]
    Variant1,
    #[serde(rename = "V2")]
    Variant2,
}
#[test]
fn test_enum_rename() {
    assert_snapshot!(TestEnumRename);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum TestEnumRenameAll {
    FieldName,
}
#[test]
fn test_enum_rename_all() {
    assert_snapshot!(TestEnumRenameAll);
}

// test enum rename variant named field
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumRenameVariantField {
    // reflectapi doesn't allow renaming tuple fields. `serde_json` seems to ignore it anyway.
    // TODO: find a way to deny this in the derive macro.
    // Variant1(#[serde(rename = "variant1_field_name")] u8, usize),
    Variant2 {
        #[serde(rename = "variant2_field_name")]
        field_name: u8,
    },
}
#[test]
fn test_enum_rename_variant_field() {
    assert_snapshot!(TestEnumRenameVariantField);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum TestEnumUntagged {
    Variant1(u8),
    Variant2 { field_name: u8 },
}
#[test]
fn test_enum_untagged() {
    assert_snapshot!(TestEnumUntagged);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
// TODO this code panics on serialize:
// see https://github.com/thepartly/reflectapi/issues/1
enum TestEnumTag {
    Variant1 { field_name: u8 },
    Variant2 { field_name: u8 },
    // Variant2(u8),
}
#[test]
fn test_enum_tag() {
    assert_snapshot!(TestEnumTag);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content")]
enum TestEnumTagContent {
    Variant1 { field_name: u8 },
    Variant2(u8),
}
#[test]
fn test_enum_tag_content() {
    assert_snapshot!(TestEnumTagContent);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
enum TestEnumTagContentRenameAll {
    Variant1 { field_name: u8 },
    Variant2(u8),
}
#[test]
fn test_enum_tag_content_rename_all() {
    assert_snapshot!(TestEnumTagContentRenameAll);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumRenameAllOnVariant {
    #[serde(rename_all = "camelCase")]
    Variant1 {
        field_name: u8,
    },
    Variant2(u8),
}
#[test]
fn test_enum_rename_all_on_variant() {
    assert_snapshot!(TestEnumRenameAllOnVariant);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipSerializeIf {
    #[serde(skip_serializing_if = "Option::is_none")]
    f: Option<u8>,
}
#[test]
fn test_struct_with_serde_skip_serialize_if() {
    assert_snapshot!(TestStructWithSerdeSkipSerializeIf);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeDefault {
    #[serde(default)]
    f: u8,
}
#[test]
fn test_struct_with_serde_default() {
    assert_snapshot!(TestStructWithSerdeDefault);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkip {
    #[serde(skip)]
    _f: u8,
}
#[test]
fn test_struct_with_serde_skip() {
    assert_snapshot!(TestStructWithSerdeSkip);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipSerialize {
    #[serde(skip_serializing)]
    _f: u8,
}
#[test]
fn test_struct_with_serde_skip_serialize() {
    assert_snapshot!(TestStructWithSerdeSkipSerialize);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipDeserialize {
    #[serde(skip_deserializing)]
    f: u8,
}
#[test]
fn test_struct_with_serde_skip_deserialize() {
    assert_snapshot!(TestStructWithSerdeSkipDeserialize);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithFieldSkip {
    Variant1 {
        #[serde(skip)]
        _f: u8,
    },
}
#[test]
fn test_enum_with_field_skip() {
    assert_snapshot!(TestEnumWithFieldSkip);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkip {
    #[serde(skip)]
    _Variant1,
}
#[test]
fn test_enum_with_variant_skip() {
    assert_snapshot!(TestEnumWithVariantSkip);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkipSerialize {
    #[serde(skip_serializing)]
    Variant1,
}
#[test]
fn test_enum_with_variant_skip_serialize() {
    assert_snapshot!(TestEnumWithVariantSkipSerialize);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkipDeserialize {
    #[serde(skip_deserializing)]
    _Variant1,
}
#[test]
fn test_enum_with_variant_skip_deserialize() {
    assert_snapshot!(TestEnumWithVariantSkipDeserialize);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
struct TestStructWithSerdeTransparent {
    _f: u8,
}
#[test]
fn test_struct_with_serde_transparent() {
    assert_snapshot!(TestStructWithSerdeTransparent);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantUntagged {
    #[serde(untagged)]
    Variant1(u8),
}
#[test]
fn test_enum_with_variant_untagged() {
    assert_snapshot!(TestEnumWithVariantUntagged);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum TestEnumWithVariantOther {
    V0,
    #[serde(other)]
    Variant1,
}
#[test]
fn test_enum_with_variant_other() {
    assert_snapshot!(TestEnumWithVariantOther);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenNested {
    f: u8,
}
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlatten {
    #[serde(flatten)]
    g: TestStructWithFlattenNested,
}
#[test]
fn test_struct_with_flatten() {
    assert_snapshot!(TestStructWithFlatten);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenOptional {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    g: Option<TestStructWithFlattenNested>,
}
#[test]
fn test_struct_with_flatten_optional() {
    assert_snapshot!(TestStructWithFlattenOptional);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenOptionalAndRequired {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    g: Option<TestStructWithFlattenNested>,
    #[serde(flatten)]
    k: TestStructRenameAll,
}
#[test]
fn test_struct_with_flatten_optional_and_required() {
    assert_snapshot!(TestStructWithFlattenOptionalAndRequired);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "struct-name&&")]
struct TestStructWithRenameToInvalidChars {
    #[serde(rename = "field-name&&")]
    f: u8,
}
#[test]
fn test_struct_with_rename_to_invalid_chars() {
    assert_snapshot!(TestStructWithRenameToInvalidChars);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "enum-name&&")]
enum TestEnumWithRenameToInvalidChars {
    #[serde(rename = "variant-name&&")]
    Variant1 {
        #[serde(rename = "field-name&&")]
        f: u8,
    },
}
#[test]
fn test_enum_with_rename_to_invalid_chars() {
    assert_snapshot!(TestEnumWithRenameToInvalidChars);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "struct-name")]
struct TestStructWithRenameToKebabCase {
    #[serde(rename = "field-name")]
    f: u8,
}
#[test]
fn test_struct_with_rename_to_kebab_case() {
    assert_snapshot!(TestStructWithRenameToKebabCase);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "TestStructTryFrom")]
struct TestStructTryFormProxy {
    f: u8,
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructTryFrom {
    f: u8,
}
impl TryFrom<TestStructTryFrom> for TestStructTryFormProxy {
    type Error = String;
    fn try_from(value: TestStructTryFrom) -> Result<Self, Self::Error> {
        Ok(TestStructTryFormProxy { f: value.f })
    }
}
#[test]
fn test_struct_try_from() {
    assert_snapshot!(TestStructTryFormProxy);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[serde(from = "TestStructFrom")]
struct TestStructFromProxy {
    f: u8,
}
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructFrom {
    f: u8,
}
impl From<TestStructFrom> for TestStructFromProxy {
    fn from(value: TestStructFrom) -> Self {
        TestStructFromProxy { f: value.f }
    }
}
#[test]
fn test_struct_from() {
    assert_snapshot!(TestStructFromProxy);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize, Clone)]
#[serde(into = "TestStructInto")]
struct TestStructIntoProxy {
    f: u8,
}
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructInto {
    f: u8,
}
impl From<TestStructIntoProxy> for TestStructInto {
    fn from(val: TestStructIntoProxy) -> Self {
        TestStructInto { f: val.f }
    }
}
#[test]
fn test_struct_into() {
    assert_snapshot!(TestStructIntoProxy);
}

#[test]
fn test_unit_struct() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct TestUnitStruct;
    assert_snapshot!(TestUnitStruct);
}

#[test]
fn test_unit_tuple_struct() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct TestUnitTupleStruct(());
    assert_snapshot!(TestUnitTupleStruct);
}

#[test]
fn test_newtype_variants_externally_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum TestNewtypeVariantsExternallyTagged {
        Int(i32),
        String(String),
        Bool(bool),
        Unit,
    }

    assert_snapshot!(TestNewtypeVariantsExternallyTagged);
}

#[test]
fn test_newtype_variants_adjacently_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    #[serde(tag = "t", content = "c")]
    enum TestNewtypeVariantsAdjacentlyTagged {
        Int(i32),
        String(String),
        Bool(bool),
        Unit,
    }

    assert_snapshot!(TestNewtypeVariantsAdjacentlyTagged);
}

#[test]
fn test_empty_variants_externally_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    enum TestEmptyVariantsExternallyTagged {
        Empty,
        EmptyUnit(),
        EmptyStruct {},
    }

    assert_snapshot!(TestEmptyVariantsExternallyTagged);
}

#[test]
fn test_empty_variants_internally_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum TestEmptyVariantsInterallyTagged {
        Empty,
        // Tuple variants are not allowed in this representation
        // EmptyUnit(),
        EmptyStruct {},
    }

    assert_snapshot!(TestEmptyVariantsInterallyTagged);
}

#[test]
fn test_empty_variants_adjacently_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "t", content = "c")]
    enum TestEmptyVariantsAdjacentlyTagged {
        Empty,
        EmptyUnit(),
        EmptyStruct {},
    }

    assert_snapshot!(TestEmptyVariantsAdjacentlyTagged);
}

#[test]
fn test_empty_variants_untagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(untagged)]
    enum TestEmptyVariantsUntagged {
        Empty,
        EmptyUnit(),
        EmptyStruct {},
    }

    assert_snapshot!(TestEmptyVariantsUntagged);
}

#[test]
fn test_newtype_variants_internally_tagged() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    #[serde(rename_all = "snake_case")]
    enum Enum {
        A(Strukt1),
        B(Strukt2),
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Strukt1 {
        a: u8,
        b: u16,
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Strukt2 {
        c: u32,
        d: u64,
    }

    assert_snapshot!(Enum);
}

#[test]
fn test_adj_repr_enum_with_untagged_variant() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum Test {
        Variant1 {
            field_name: u8,
        },
        #[serde(untagged)]
        Variant2(String),
    }
    assert_snapshot!(Test);
}

#[test]
fn test_flatten_unit() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    pub(crate) struct S<Payload, Additional = ()> {
        #[serde(flatten)]
        pub payload: Payload,
        #[serde(flatten)]
        pub additional: Additional,
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct K {
        a: u8,
    }

    assert_snapshot!(S<K>);
}

#[test]
fn test_flatten_internally_tagged() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    pub(crate) struct S<Payload, Additional = ()> {
        #[serde(flatten)]
        pub payload: Payload,
        #[serde(flatten)]
        pub additional: Additional,
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct A {
        a: u8,
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct B {
        b: u8,
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type")]
    enum Test {
        S(S<A, B>),
    }

    assert_snapshot!(Test);
}

#[test]
fn test_struct_repr_transparent_generic_inner_type() {
    #[derive(serde::Deserialize, serde::Serialize, reflectapi::Input, reflectapi::Output)]
    #[serde(transparent)]
    pub struct Inner(pub std::collections::HashSet<u8>);

    #[derive(serde::Deserialize, serde::Serialize, reflectapi::Input, reflectapi::Output)]
    pub struct Test {
        pub inner: Inner,
    }

    assert_snapshot!(Test);
}

#[test]
fn test_generic_struct_repr_transparent() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(transparent)]
    struct TestStruct<T>(Vec<T>);

    assert_snapshot!(TestStruct<u8>);
}

#[test]
fn test_generic_struct_repr_transparent_partially_generic() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(transparent)]
    struct TestStruct<V>(std::collections::HashMap<String, V>);

    assert_snapshot!(TestStruct<u8>);
}

#[test]
fn test_nested_internally_tagged_enums_minimal() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "version")]
    #[serde(rename_all = "snake_case")]
    enum Test {
        V1(V1),
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum V1 {
        A { a: u8 },
    }

    assert_snapshot!(Test);
}

#[test]
fn test_nested_internally_tagged_enums() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "version")]
    #[serde(rename_all = "snake_case")]
    enum Test {
        V1(V1),
        V2(V2),
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum V1 {
        A { a: u8 },
        B { b: u16 },
    }

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "type")]
    enum V2 {
        C { c: u32 },
        D { d: u64 },
    }

    assert_snapshot!(Test);
}

#[test]
fn test_datetime() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct TestStruct {
        duration: std::time::Duration,
        naive_time: chrono::NaiveTime,
        naive_date: chrono::NaiveDate,
        naive_datetime: chrono::NaiveDateTime,
        date_time_fixed_offset: chrono::DateTime<chrono::FixedOffset>,
        date_time_utc: chrono::DateTime<chrono::Utc>,
        date_time_local: chrono::DateTime<chrono::Local>,
    }

    assert_snapshot!(TestStruct);
}

#[test]
fn test_timezone() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct TestStruct {
        timezone: chrono_tz::Tz,
    }

    assert_snapshot!(TestStruct);
}

#[test]
fn test_external_impls() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    struct Test {
        index_map: indexmap::IndexMap<u8, u32>,
        index_set: indexmap::IndexSet<String>,
        url: url::Url,
        json: serde_json::Value,
    }

    assert_snapshot!(Test);
}
