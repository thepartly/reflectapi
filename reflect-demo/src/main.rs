#[cfg(test)]
mod test;
mod test_lib;

// #[derive(reflect::Input, reflect::Output)]
// struct MyStruct {
//     /// some docs
//     // #[serde(flatten)]
//     // #[reflect(invalid)]
//     _f: u32,
//     _f2: i8,
// }

// #[derive(reflect::Input, reflect::Output)]
// struct TestStructWithNested {
//     _f: TestStructNested,
// }
// #[derive(reflect::Input, reflect::Output)]
// struct TestStructNested {
//     _f: String,
// }

#[derive(reflect::Input)]
struct TestStructWithVec {
    _f: Vec<u8>,
}

trait MyTrait {}

// impl<T: serde::Serialize> MyTrait for T {}
// impl<'de, T: serde::Deserialize<'de>> MyTrait for T {}

fn main() {
    // println!("{}", MyStruct::reflect_input());
    println!("{}", TestStructWithVec::reflect_input());
}
