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

#[derive(reflect::Input)]
struct ParentStruct {
    _f: GenericStruct<GenericStruct<u8>>,
}

#[derive(reflect::Input)]
struct GenericStruct<A>
where
    A: reflect::Input,
{
    _f1: A,
}

fn main() {
    // //println!("{}", MyStruct::reflect_input());
    // //println!(
    //     "{:#?}",
    //     GenericStruct::<GenericStruct::<u8>>::reflect_input()
    // );
    //println!("{:#?}", ParentStruct::reflect_input());

    // //println!(
    //     "{:#?}",
    //     TestStructWithCircularReferenceGenericWithoutBox::<
    //         TestStructWithCircularReferenceGenericWithoutBox::<u8>,
    //     >::reflect_input()
    // );
}
