#[derive(serde::Serialize, reflectapi::Output)]
struct NotAnError {}

async fn handler(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, NotAnError> {
    Ok(reflectapi::Empty {})
}

fn main() {
    let _ = reflectapi::Builder::<()>::new().route(handler, |b| b.name("check"));
}
