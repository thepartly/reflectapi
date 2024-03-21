#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithSimpleGeneric<A>
where
    A: reflect::Input,
{
    _f: A,
}
#[test]
fn test_struct_with_simple_generic() {
    assert_input_snapshot!(TestStructWithSimpleGeneric::<u8>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithVecGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<T>,
}
#[test]
fn test_struct_with_vec_generic() {
    assert_input_snapshot!(TestStructWithVecGeneric::<u8>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithVecGenericGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<TestStructWithSimpleGeneric<T>>,
}
#[test]
fn test_struct_with_vec_generic_generic() {
    assert_input_snapshot!(TestStructWithVecGenericGeneric::<u8>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithVecGenericGenericGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<TestStructWithVecGeneric<T>>,
}
#[test]
fn test_struct_with_vec_generic_generic_generic() {
    assert_input_snapshot!(TestStructWithVecGenericGenericGeneric::<Vec<u8>>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithNestedGenericStruct {
    _f: TestStructWithSimpleGeneric<TestStructWithSimpleGeneric<u8>>,
}
#[test]
fn test_struct_with_nested_generic_struct() {
    assert_input_snapshot!(TestStructWithNestedGenericStruct);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithNestedGenericStructTwice {
    _f: TestStructWithSimpleGeneric<u8>,
    _f2: TestStructWithSimpleGeneric<String>,
}
#[test]
fn test_struct_with_nested_generic_struct_twice() {
    assert_input_snapshot!(TestStructWithNestedGenericStructTwice);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReference {
    _f: Box<TestStructWithCircularReference>,
}
#[test]
fn test_struct_with_circular_reference() {
    assert_input_snapshot!(TestStructWithCircularReference);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReferenceGeneric<T>
where
    T: reflect::Input,
{
    _f: Box<TestStructWithCircularReferenceGeneric<T>>,
    _f2: T,
}
#[test]
fn test_struct_with_circular_reference_generic() {
    assert_input_snapshot!(TestStructWithCircularReferenceGeneric::<u8>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReferenceGenericParent<T>
where
    T: reflect::Input,
{
    _f: Box<
        TestStructWithCircularReferenceGeneric<TestStructWithCircularReferenceGenericParent<T>>,
    >,
    _f2: std::marker::PhantomData<T>,
}
#[test]
fn test_struct_with_circular_reference_generic_parent() {
    assert_input_snapshot!(TestStructWithCircularReferenceGenericParent::<u8>);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReferenceGenericWithoutBox<A, B>
where
    A: reflect::Input,
    B: reflect::Input,
{
    _f1: A,
    _f2: B,
}
#[test]
fn test_struct_with_circular_reference_generic_without_box() {
    assert_input_snapshot!(TestStructWithCircularReferenceGenericWithoutBox::<
        TestStructWithCircularReferenceGenericWithoutBox<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBox<String, u32>,
    >);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReferenceGenericWithoutBoxParent<C, D>
where
    C: reflect::Input,
    D: reflect::Input,
{
    _f: TestStructWithCircularReferenceGenericWithoutBox<D, C>,
}
#[test]
fn test_struct_with_circular_reference_generic_without_box_parent() {
    assert_input_snapshot!(TestStructWithCircularReferenceGenericWithoutBoxParent::<
        TestStructWithCircularReferenceGenericWithoutBoxParent<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBoxParent<String, u32>,
    >);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructWithCircularReferenceGenericWithoutBoxParentSpecific {
    _f: TestStructWithCircularReferenceGenericWithoutBox<
        TestStructWithCircularReferenceGenericWithoutBox<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBox<String, u32>,
    >,
}
#[test]
fn test_struct_with_circular_reference_generic_without_box_parent_specific() {
    assert_input_snapshot!(
        TestStructWithCircularReferenceGenericWithoutBoxParentSpecific
    );
}
