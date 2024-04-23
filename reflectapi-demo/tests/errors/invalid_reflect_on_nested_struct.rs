#[derive(reflectapi::Input, reflectapi::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
// #[derive(reflectapi::Input, reflectapi::Output)]
struct TestStructNested {
    _f: String,
}

fn main() {}
