#[derive(Debug, serde::Serialize, reflectapi::Input, reflectapi::Output)]
pub struct Gen<T: reflectapi::Output = ()>(T);

fn main() {}
