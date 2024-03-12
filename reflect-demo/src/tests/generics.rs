#[derive(reflect::Input)]
struct TestStructWithSimpleGeneric<A>
where
    A: reflect::Input,
{
    _f: A,
}
#[test]
fn test_struct_with_simple_generic() {
    insta::assert_json_snapshot!(TestStructWithSimpleGeneric::<u8>::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithVecGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<T>,
}
#[test]
fn test_struct_with_vec_generic() {
    insta::assert_json_snapshot!(TestStructWithVecGeneric::<u8>::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithVecGenericGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<TestStructWithSimpleGeneric<T>>,
}
#[test]
fn test_struct_with_vec_generic_generic() {
    insta::assert_json_snapshot!(TestStructWithVecGenericGeneric::<u8>::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithVecGenericGenericGeneric<T>
where
    T: reflect::Input,
{
    _f: Vec<TestStructWithVecGeneric<T>>,
}
#[test]
fn test_struct_with_vec_generic_generic_generic() {
    insta::assert_json_snapshot!(
        TestStructWithVecGenericGenericGeneric::<Vec<u8>>::reflect_input()
    );
}

#[derive(reflect::Input)]
struct TestStructWithNestedGenericStruct {
    _f: TestStructWithSimpleGeneric<TestStructWithSimpleGeneric<u8>>,
}
#[test]
fn test_struct_with_nested_generic_struct() {
    insta::assert_json_snapshot!(TestStructWithNestedGenericStruct::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithNestedGenericStructTwice {
    _f: TestStructWithSimpleGeneric<u8>,
    _f2: TestStructWithSimpleGeneric<String>,
}
#[test]
fn test_struct_with_nested_generic_struct_twice() {
    insta::assert_json_snapshot!(TestStructWithNestedGenericStructTwice::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithCircularReference {
    _f: Box<TestStructWithCircularReference>,
}
#[test]
fn test_struct_with_circular_reference() {
    insta::assert_json_snapshot!(TestStructWithCircularReference::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithCircularReferenceGeneric<T>
where
    T: reflect::Input,
{
    _f: Box<TestStructWithCircularReferenceGeneric<T>>,
    _f2: T,
}
#[test]
fn test_struct_with_circular_reference_generic() {
    insta::assert_json_snapshot!(TestStructWithCircularReferenceGeneric::<u8>::reflect_input());
}

#[derive(reflect::Input)]
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
    insta::assert_json_snapshot!(
        TestStructWithCircularReferenceGenericParent::<u8>::reflect_input()
    );
}

#[derive(reflect::Input)]
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
    insta::assert_json_snapshot!(TestStructWithCircularReferenceGenericWithoutBox::<
        TestStructWithCircularReferenceGenericWithoutBox<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBox<String, u32>,
    >::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithCircularReferenceGenericWithoutBoxParent<C, D>
where
    C: reflect::Input,
    D: reflect::Input,
{
    _f: TestStructWithCircularReferenceGenericWithoutBox<D, C>,
}
#[test]
fn test_struct_with_circular_reference_generic_without_box_parent() {
    insta::assert_json_snapshot!(TestStructWithCircularReferenceGenericWithoutBoxParent::<
        TestStructWithCircularReferenceGenericWithoutBoxParent<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBoxParent<String, u32>,
    >::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructWithCircularReferenceGenericWithoutBoxParentSpecific {
    _f: TestStructWithCircularReferenceGenericWithoutBox<
        TestStructWithCircularReferenceGenericWithoutBox<u8, u16>,
        TestStructWithCircularReferenceGenericWithoutBox<String, u32>,
    >,
}
#[test]
fn test_struct_with_circular_reference_generic_without_box_parent_specific() {
    insta::assert_json_snapshot!(
        TestStructWithCircularReferenceGenericWithoutBoxParentSpecific::reflect_input()
    );
}

