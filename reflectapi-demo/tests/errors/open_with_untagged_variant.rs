#[derive(reflectapi::Output)]
#[reflectapi(open)]
enum Value {
    Known,
    #[serde(untagged)]
    CatchAll(String),
}

fn main() {}
