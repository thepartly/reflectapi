#[derive(reflectapi::Input)]
struct TestStructWithNested {
    _f: TestStructNested,
    _f2: TestStructNested,
}
struct TestStructNested {
    _f: String,
}

fn main() {}
