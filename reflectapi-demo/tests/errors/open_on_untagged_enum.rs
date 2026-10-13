#[derive(reflectapi::Output)]
#[serde(untagged)]
#[reflectapi(open)]
enum Value {
    Text(String),
    Number(u32),
}

fn main() {}
