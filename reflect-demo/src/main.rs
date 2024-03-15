#[cfg(test)]
mod tests;

// #[derive(serde::Serialize)]
// struct A {
//     _a: std::marker::PhantomData<MyStruct<u8>>,
// }

// struct MyStruct<T> {
//     f2: T,
//     f: Box<MyStruct<T>>,
// }

// #[derive(reflect::Input)]
// struct TestStructWithVec<T>
// where
//     T: reflect::Input,
// {
//     _f: T,
// }

// #[derive(reflect::Input)]
// struct TestStructParent
// // where
// //     T: reflect::Input,
// {
//     _f: TestStructWithVec<u8>,
//     // _f2: TestStructWithVec<T>,
// }

// #[derive(serde::Deserialize)]
// struct Test<'a> {
//     _a: std::slice::Iter<'a, u8>,
// }

// trait MyTrait {}

// #[derive(reflect::Input)]
// struct ParentStruct {
//     _f: GenericStruct<GenericStruct<u8>>,
// }

// #[derive(reflect::Input)]
// struct GenericStruct<A>
// where
//     A: reflect::Input,
// {
//     _f1: A,
// }

/// Some Enum docs
// /// more
// #[allow(unused_doc_comments, dead_code)]
// #[derive(reflect::Input)]
// enum MyEnum<
//     /// some generic param docs
//     /// multiline
//     T,
// > where
//     T: reflect::Input,
// {
//     /// Variant1 docs
//     Variant1(
//         /// variant1 field docs
//         T,
//     ),
//     /// Variant2 docs
//     /// multiline
//     /// more
//     /// more
//     Variant2 {
//         /// named field variant2 field docs
//         named_field: T,
//     },
// }

/// Some Struct docs
/// more
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflect::Input)]
struct TestStructDocumented {
    /// field docs
    /// multiline
    f: uuid::Uuid,
}

// #[derive(reflect::Input)]
// union MyUnion {
//     f: u8,
// }

fn main() {
    // println!("{:#?}", TypeAlias::reflect_input());
    // //println!(
    //     "{:#?}",
    //     GenericStruct::<GenericStruct::<u8>>::reflect_input()
    // );
    println!("{:#?}", TestStructDocumented::reflect_input());

    // //println!(
    //     "{:#?}",
    //     TestStructWithCircularReferenceGenericWithoutBox::<
    //         TestStructWithCircularReferenceGenericWithoutBox::<u8>,
    //     >::reflect_input()
    // );
}
