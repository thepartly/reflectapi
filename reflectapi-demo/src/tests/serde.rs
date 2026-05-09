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

/// Regression test for issue #123: flattened internally-tagged enum fields
/// were silently dropped by the Python codegen.
#[test]
fn test_flatten_internally_tagged_enum_field() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type")]
    enum OfferKind {
        Single { business: String },
        Group { count: u32 },
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Offer {
        id: String,
        #[serde(flatten)]
        payload: OfferKind,
    }

    assert_snapshot!(Offer);
}

#[test]
fn test_flatten_externally_tagged_enum_field() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    enum Shape {
        Circle { radius: f64 },
        Rect { width: f64, height: f64 },
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Drawing {
        name: String,
        #[serde(flatten)]
        shape: Shape,
    }

    assert_snapshot!(Drawing);
}

#[test]
fn test_flatten_adjacently_tagged_enum_field() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "kind", content = "data")]
    enum Payload {
        Text { body: String },
        Binary { size: u32 },
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Message {
        id: String,
        #[serde(flatten)]
        payload: Payload,
    }

    assert_snapshot!(Message);
}

#[test]
fn test_flatten_untagged_enum_field() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(untagged)]
    enum Value {
        Num { value: f64 },
        Text { text: String },
    }

    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Cell {
        label: String,
        #[serde(flatten)]
        content: Value,
    }

    assert_snapshot!(Cell);
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

// ──────────────────────────────────────────────────────────────────────
// Group 1: Namespace Edge Cases
// ──────────────────────────────────────────────────────────────────────

#[test]
fn test_namespace_single_segment_type() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct SimpleTopLevel {
        value: u32,
    }
    assert_snapshot!(SimpleTopLevel);
}

#[test]
fn test_namespace_deeply_nested_modules() {
    mod deep {
        pub mod nested {
            pub mod inner {
                #[derive(
                    serde::Serialize,
                    serde::Deserialize,
                    Debug,
                    reflectapi::Input,
                    reflectapi::Output,
                )]
                pub struct DeepType {
                    pub data: String,
                }
            }
        }
    }
    assert_snapshot!(deep::nested::inner::DeepType);
}

#[test]
fn test_namespace_with_numeric_start() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct TypeWithNumbers {
        #[serde(rename = "123start")]
        field_123: String,
        #[serde(rename = "kebab-field")]
        kebab_field: u32,
    }
    assert_snapshot!(TypeWithNumbers);
}

// ──────────────────────────────────────────────────────────────────────
// Group 2: Flatten Edge Cases
// ──────────────────────────────────────────────────────────────────────

#[test]
fn test_flatten_struct_with_nested_flatten() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Inner {
        z: String,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Middle {
        y: u32,
        #[serde(flatten)]
        inner: Inner,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Outer {
        x: String,
        #[serde(flatten)]
        nested: Middle,
    }
    assert_snapshot!(Outer);
}

#[test]
fn test_flatten_optional_internally_tagged_enum() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "kind")]
    enum Priority {
        High { deadline: String },
        Low,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Task {
        title: String,
        #[serde(flatten)]
        priority: Option<Priority>,
    }
    assert_snapshot!(Task);
}

#[test]
fn test_flatten_multiple_structs() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Timestamps {
        created_at: String,
        updated_at: String,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Metadata {
        author: String,
        version: u32,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Document {
        title: String,
        #[serde(flatten)]
        timestamps: Timestamps,
        #[serde(flatten)]
        meta: Metadata,
    }
    assert_snapshot!(Document);
}

#[test]
fn test_flatten_struct_and_internal_enum_combined() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Audit {
        modified_by: String,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type")]
    enum Content {
        Text { body: String },
        Image { url: String, width: u32 },
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Post {
        id: String,
        #[serde(flatten)]
        audit: Audit,
        #[serde(flatten)]
        content: Content,
    }
    assert_snapshot!(Post);
}

#[test]
fn test_flatten_enum_with_unit_variants_only() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "status")]
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Item {
        name: String,
        #[serde(flatten)]
        status: Status,
    }
    assert_snapshot!(Item);
}

// ──────────────────────────────────────────────────────────────────────
// Group 3: Enum Representation Edge Cases
// ──────────────────────────────────────────────────────────────────────

#[test]
fn test_generic_externally_tagged_enum() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    enum Wrapper<T: reflectapi::Input + reflectapi::Output> {
        Value(T),
        Empty,
    }
    assert_snapshot!(Wrapper<String>);
}

#[test]
fn test_generic_adjacently_tagged_enum() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "t", content = "c")]
    enum Tagged<T: reflectapi::Input + reflectapi::Output> {
        Item(T),
        Nothing,
    }
    assert_snapshot!(Tagged<u32>);
}

#[test]
fn test_enum_mixed_variant_types_internally_tagged() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type")]
    enum Mixed {
        Unit,
        // Note: tuple variants are not allowed in internally-tagged representation,
        // so we use a struct variant instead.
        Wrap { value: String },
        Full { x: i32, y: i32 },
    }
    assert_snapshot!(Mixed);
}

#[test]
fn test_enum_with_serde_rename_on_variants() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "kind", content = "data")]
    enum Action {
        #[serde(rename = "create_item")]
        Create { name: String },
        #[serde(rename = "delete_item")]
        Delete { id: u32 },
    }
    assert_snapshot!(Action);
}

// ── Group 4: Type Reference Edge Cases ──────────────────────────────────────

#[test]
fn test_box_field_unwrapping() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct TreeNode {
        label: String,
        child: Option<Box<TreeNode>>,
    }
    assert_snapshot!(TreeNode);
}

#[test]
fn test_nested_generic_containers() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Complex {
        matrix: Vec<Vec<u32>>,
        lookup: std::collections::HashMap<String, Vec<Option<i32>>>,
    }
    assert_snapshot!(Complex);
}

#[test]
fn test_self_referential_struct() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Category {
        name: String,
        subcategories: Vec<Category>,
    }
    assert_snapshot!(Category);
}

#[test]
fn test_option_of_option() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Nested {
        value: Option<Option<String>>,
    }
    assert_snapshot!(Nested);
}

// ── Group 5: Field Sanitization Edge Cases ──────────────────────────────────

#[test]
fn test_field_all_python_keywords() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Keywords {
        #[serde(rename = "type")]
        type_field: String,
        #[serde(rename = "class")]
        class_field: String,
        #[serde(rename = "from")]
        from_field: String,
        #[serde(rename = "import")]
        import_field: String,
    }
    assert_snapshot!(Keywords);
}

#[test]
fn test_field_names_with_special_chars() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct SpecialNames {
        #[serde(rename = "Content-Type")]
        content_type: String,
        #[serde(rename = "x.nested.key")]
        nested_key: String,
        #[serde(rename = "has spaces")]
        has_spaces: u32,
    }
    assert_snapshot!(SpecialNames);
}

#[test]
fn test_multiple_underscore_prefix_fields() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    struct Underscored {
        _single: u32,
        __double: String,
        ___triple: bool,
    }
    assert_snapshot!(Underscored);
}

// ── Group 6: Factory & Client Edge Cases ────────────────────────────────────

#[test]
fn test_enum_with_many_variants() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type")]
    enum LargeEnum {
        Alpha,
        Beta,
        Gamma { x: i32 },
        Delta { value: String },
        Epsilon,
        Zeta { y: bool },
        Eta,
        Theta { value: u32 },
        Iota,
        Kappa { z: f64 },
        Lambda,
        Mu { w: String, v: i32 },
    }
    assert_snapshot!(LargeEnum);
}

#[test]
fn test_empty_enum() {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    enum Never {}
    assert_snapshot!(Never);
}

// Reproducer: a generic wrapper that flattens its generic parameter.
// Pattern used in real consumer code (e.g. UpdateOrElse<T, C>).
//
// Wire format with serde(flatten) on `inner: T` is: T's fields ⊕ if_field
// at the top level. Python codegen previously dropped T's fields entirely
// because schema.get_type("T") returns None for a TypeVar.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestFlattenInner {
    inner_a: u32,
    inner_b: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestFlattenInnerAlt {
    alt_x: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestFlattenIfElse {
    code: u16,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "T: serde::Serialize, C: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned, C: serde::de::DeserializeOwned",
))]
struct TestUpdateOrElse<T, C> {
    #[serde(flatten)]
    inner: T,
    if_else: Option<C>,
}

// Two endpoints sharing the same generic wrapper but with different
// instantiations. Each must produce its own monomorphized class.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestTwoInstantiations {
    a: TestUpdateOrElse<TestFlattenInner, TestFlattenIfElse>,
    b: TestUpdateOrElse<TestFlattenInnerAlt, TestFlattenIfElse>,
}

// Optional-wrapped flatten of a generic param: serde unwraps Option in
// flatten position, so wire shape = T's fields ⊕ if_else (but T may
// be missing entirely). Verify codegen still expands T's fields.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned",
))]
struct TestOptionalFlatten<T> {
    #[serde(flatten, default = "Option::default")]
    inner: Option<T>,
    code: u16,
}

#[test]
fn test_generic_flatten_drops_inner_fields() {
    assert_snapshot!(TestUpdateOrElse<TestFlattenInner, TestFlattenIfElse>);
}

#[test]
fn test_generic_flatten_two_instantiations() {
    assert_snapshot!(TestTwoInstantiations);
}

#[test]
fn test_generic_flatten_optional() {
    assert_snapshot!(TestOptionalFlatten<TestFlattenInner>);
}

// Real-world pattern: an outer generic wrapper flattens an *inner*
// generic wrapper. UpdateOrElse<IdentityData<I, D>, C> where
// IdentityData itself is a marked struct (flattens its own generic
// I and D). Codegen must monomorphize bottom-up.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestFlattenIdent {
    job_id: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestFlattenIdentData {
    payload: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "I: serde::Serialize, D: serde::Serialize",
    deserialize = "I: serde::de::DeserializeOwned, D: serde::de::DeserializeOwned",
))]
struct TestIdentityData<I, D> {
    #[serde(flatten)]
    identity: I,
    #[serde(flatten)]
    data: D,
}

#[test]
fn test_generic_flatten_nested() {
    assert_snapshot!(
        TestUpdateOrElse<
            TestIdentityData<TestFlattenIdent, TestFlattenIdentData>,
            TestFlattenIfElse,
        >
    );
}

/// End-to-end Pydantic round-trip: confirms the generated class
/// actually parses serde's wire format. Skipped if `uv` (or a Python
/// with pydantic) isn't available locally — CI uses uv.
#[test]
fn test_generic_flatten_pydantic_roundtrip() {
    use std::io::Write;

    let py_source =
        super::into_python_code::<TestUpdateOrElse<TestFlattenInner, TestFlattenIfElse>>();

    // Wire format that serde would actually produce / accept: T's
    // fields ⊕ if_else, all at top level.
    let wire_payload =
        serde_json::to_string(&TestUpdateOrElse::<TestFlattenInner, TestFlattenIfElse> {
            inner: TestFlattenInner {
                inner_a: 7,
                inner_b: "hello".into(),
            },
            if_else: Some(TestFlattenIfElse { code: 409 }),
        })
        .unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let module_path = tmp.path().join("generated.py");
    std::fs::write(&module_path, py_source).unwrap();

    // The mangled class name depends on namespace and length-budget
    // hashing, so the test discovers it by walking the generated
    // namespace and matching on the expected field set.
    let driver = format!(
        r#"
import importlib.util, json
from pydantic import BaseModel
spec = importlib.util.spec_from_file_location("gen", r"{module}")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
ns = m.reflectapi_demo.tests.serde
expected = {{"inner_a", "inner_b", "if_else"}}
candidates = [
    getattr(ns, attr)
    for attr in dir(ns)
    if isinstance(getattr(ns, attr, None), type)
    and issubclass(getattr(ns, attr), BaseModel)
    and set(getattr(ns, attr).model_fields.keys()) == expected
]
assert len(candidates) == 1, (
    "expected exactly one class with fields {{inner_a, inner_b, if_else}}; "
    "got " + repr([c.__name__ for c in candidates])
)
cls = candidates[0]
parsed = cls.model_validate(json.loads(r'''{payload}'''))
assert parsed.inner_a == 7, ("inner_a", parsed.inner_a)
assert parsed.inner_b == "hello", ("inner_b", parsed.inner_b)
assert parsed.if_else is not None and parsed.if_else.code == 409, ("if_else", parsed.if_else)
# Round-trip back through json: model_dump returns a dict; serialize and
# verify the wire format still has the flatten shape (T's fields at top).
out = parsed.model_dump(by_alias=True, exclude_none=False)
assert out["inner_a"] == 7
assert out["inner_b"] == "hello"
assert out["if_else"]["code"] == 409
print("OK")
"#,
        module = module_path.display(),
        payload = wire_payload,
    );

    // Prefer `uv run python` from the python-runtime workspace (which
    // declares pydantic as a dep). Fall back to bare `python3` if it
    // already has pydantic on its path. Skip the test only if neither
    // works — CI installs uv so it'll always exercise this.
    let runtime_dir = format!(
        "{}/../reflectapi-python-runtime",
        env!("CARGO_MANIFEST_DIR")
    );

    let mut attempts: Vec<(String, std::process::Command)> = Vec::new();
    {
        let mut c = std::process::Command::new("uv");
        c.args(["run", "--directory", &runtime_dir, "python", "-"]);
        attempts.push(("uv run python".to_string(), c));
    }
    if std::process::Command::new("python3")
        .args(["-c", "import pydantic"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        let mut c = std::process::Command::new("python3");
        c.args(["-"]);
        attempts.push(("system python3+pydantic".to_string(), c));
    }

    for (label, mut cmd) in attempts {
        let spawn = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let mut child = match spawn {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skip via {label}: spawn failed ({e})");
                continue;
            }
        };
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(driver.as_bytes()).unwrap();
        }
        let output = child.wait_with_output().expect("python wait");
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("OK"),
                "{label}: missing OK marker.\nstdout:\n{stdout}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stderr),
            );
            eprintln!("{label}: pydantic roundtrip OK");
            return;
        }
        // Distinguish "no pydantic" (skip) from "pydantic is there but
        // assertions failed" (fail). We marshal the assertion error
        // through the script's exit; pydantic missing is an
        // ImportError before our prints fire.
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("ModuleNotFoundError") && stderr.contains("pydantic") {
            eprintln!("skip via {label}: pydantic not installed");
            continue;
        }
        panic!(
            "{label}: roundtrip failed (exit={:?})\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            stderr,
        );
    }
    eprintln!("test_generic_flatten_pydantic_roundtrip skipped: no working python+pydantic found");
}

// Two args that share a *leaf* name but live in different modules
// must NOT collide when mangled. Earlier mangling used only the leaf
// name; both ./module_a::Sample and ./module_b::Sample produced
// identical keys, fusing two distinct UpdateOrElse instantiations
// into one and losing one type's wire fields.
mod module_a {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Sample {
        pub a_field: u32,
    }
}
mod module_b {
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Sample {
        pub b_field: bool,
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
struct TestLeafCollisionPair {
    a: TestUpdateOrElse<module_a::Sample, TestFlattenIfElse>,
    b: TestUpdateOrElse<module_b::Sample, TestFlattenIfElse>,
}

#[test]
fn test_generic_flatten_leaf_collision() {
    assert_snapshot!(TestLeafCollisionPair);
}

// Wrapper that USES a marked struct in generic position with its own
// TypeVars as args. Reproduces the case the previous monomorphizer
// tripped over: walking `IdentityData<I, D>` inside a generic context
// where `I` and `D` are TypeVars (not real types). Should not
// register a monomorphization for that — only the concrete
// instantiation `WithMarkedInner<TestFlattenIdent, TestFlattenIdentData>`
// triggers monomorphization, and its substituted IdentityData ref is
// the one that gets a concrete monomorph.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "I: serde::Serialize, D: serde::Serialize",
    deserialize = "I: serde::de::DeserializeOwned, D: serde::de::DeserializeOwned",
))]
struct TestWithMarkedInner<I, D> {
    body: TestIdentityData<I, D>,
    extra: bool,
}

#[test]
fn test_generic_flatten_typevar_in_generic_context() {
    assert_snapshot!(TestWithMarkedInner<TestFlattenIdent, TestFlattenIdentData>);
}

// Generic enum whose variants reference a marked struct using the
// enum's own TypeVars. Reproduces the regression where transitive
// marking only considered structs — generic enums would survive the
// pass while the marked structs they referenced got removed,
// dangling the enum's variant fields.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "I: serde::Serialize, D: serde::Serialize",
    deserialize = "I: serde::de::DeserializeOwned, D: serde::de::DeserializeOwned",
))]
enum TestIngestRelation<I, D> {
    Insert(TestIdentityData<I, D>),
    Remove(TestIdentityData<I, D>),
    Empty,
}

#[test]
fn test_generic_flatten_enum_variant_typevar() {
    assert_snapshot!(TestIngestRelation<TestFlattenIdent, TestFlattenIdentData>);
}

// Note: a Rust-derive recursive marked struct (e.g.
// `struct Tree<T> { #[flatten] value: T, children: Vec<Tree<T>> }`)
// overflows the reflectapi derive macro during schema construction,
// before any codegen runs — that's a derive-side limitation, not a
// codegen one. The codegen pipeline itself handles a recursive
// marked struct fed in via JSON; that case is exercised by
// `recursive_marked_struct_terminates_and_renders` in
// reflectapi/src/codegen/python.rs.

// Boundary case: a generic struct whose flatten target is a
// *concrete* type, not a TypeVar. Should NOT be marked, NOT
// monomorphized — the existing flatten-of-concrete rendering path
// handles it. Confirms the marked-detection predicate doesn't
// over-trigger on any-flatten-on-a-generic-struct.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned",
))]
struct TestGenericWithConcreteFlatten<T> {
    #[serde(flatten)]
    extra: TestFlattenInner,
    other: T,
}

#[test]
fn test_generic_with_concrete_flatten_not_marked() {
    assert_snapshot!(TestGenericWithConcreteFlatten<TestFlattenIfElse>);
}

// Marked struct used with a generic parameter wrapped inside another
// generic (`Marked<Option<I>>`). Earlier transitive marking only
// looked at the IMMEDIATE arg, so the wrapper wasn't marked, the
// inner Marked got removed, and the wrapper's ref dangled.
#[derive(serde::Serialize, serde::Deserialize, Debug, reflectapi::Input, reflectapi::Output)]
#[serde(bound(
    serialize = "I: serde::Serialize",
    deserialize = "I: serde::de::DeserializeOwned",
))]
struct TestWrapperWithNestedTypevarArg<I> {
    body: TestUpdateOrElse<Option<I>, TestFlattenIfElse>,
    extra: u32,
}

#[test]
fn test_generic_flatten_typevar_nested_in_generic_arg() {
    assert_snapshot!(TestWrapperWithNestedTypevarArg<TestFlattenInner>);
}
