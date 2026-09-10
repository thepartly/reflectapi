#[derive(serde::Serialize, reflectapi::Output)]
struct Response(#[reflectapi(header)] String);

fn main() {}
