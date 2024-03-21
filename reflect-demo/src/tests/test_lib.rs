// #[cfg(test)]
#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
pub struct TestStructNested {
    _f: String,
}
