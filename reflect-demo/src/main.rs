#[cfg(test)]
mod tests;

#[derive(reflect::Input)]
struct TestStructWithVec<T>
where
    T: reflect::Input,
{
    _f: T,
}

#[derive(reflect::Input)]
struct TestStructParent
// where
//     T: reflect::Input,
{
    _f: TestStructWithVec<u8>,
    // _f2: TestStructWithVec<T>,
}

// #[derive(serde::Deserialize)]
// struct Test<'a> {
//     _a: std::slice::Iter<'a, u8>,
// }

trait MyTrait {}

fn main() {
    // println!("{}", MyStruct::reflect_input());
    println!("{:#?}", TestStructParent::reflect_input());
    // println!("{:#?}", TestStructWithVec::<u8>::reflect_input());
}
