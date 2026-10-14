#[derive(reflectapi::Output)]
#[reflectapi(open, discriminant)]
enum Value {
    Known = 1,
}

fn main() {}
