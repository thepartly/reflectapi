// #[cfg(test)]
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
pub struct TestStructNested {
    _f: String,
}
