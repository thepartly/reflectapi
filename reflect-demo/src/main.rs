#[cfg(test)]
mod test;
#[cfg(test)]
mod test_lib;

#[derive(reflect::Input)]
struct TestStructWithVec {
    _f: Vec<u8>,
}

trait MyTrait {}

fn main() {
    // println!("{}", MyStruct::reflect_input());
    println!("{:#?}", TestStructWithVec::reflect_input());
}
