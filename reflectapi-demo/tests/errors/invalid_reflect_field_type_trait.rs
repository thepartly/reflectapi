trait MyTrait {
    fn my_method(&self, s: &str) -> i32;
}

#[derive(reflectapi::Input)]
struct MyStruct {
    field: dyn MyTrait,
}

fn main() {}
