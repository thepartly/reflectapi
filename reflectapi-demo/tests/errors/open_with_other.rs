#[derive(reflectapi::Output)]
#[serde(tag = "kind")]
#[reflectapi(open)]
enum Value {
    Known,
    #[serde(other)]
    Other,
}

fn main() {}
