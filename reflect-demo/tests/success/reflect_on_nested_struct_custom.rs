#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
struct TestStructNested {
    _f: String,
}
impl reflect::Input for TestStructNested {
    fn reflect_input() -> reflect::Schema {
        reflect::Schema::new()
    }
}
impl reflect::Output for TestStructNested {
    fn reflect_output() -> reflect::Schema {
        reflect::Schema::new()
    }
}

fn main() {}
