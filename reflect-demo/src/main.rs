#[cfg(test)]
mod test;
#[cfg(test)]
mod test_lib;

#[derive(reflect::Input)]
struct TestStructWithVec<T>
where
    T: reflect::Input,
{
    _f: Vec<Vec<T>>,
}

#[derive(reflect::Input)]
struct TestStructParent {
    _f: TestStructWithVec<u8>,
}

trait MyTrait {}

fn main() {
    // println!("{}", MyStruct::reflect_input());
    println!("{:#?}", TestStructParent::reflect_input());
    // println!("{:#?}", TestStructWithVec::<u8>::reflect_input());
}
