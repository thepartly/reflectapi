#[derive(reflectapi::Input, reflectapi::Output)]
struct MyStruct<'a> {
    field: std::borrow::Cow<'a, u8>,
}

#[allow(dead_code)]
#[derive(reflectapi::Input)]
enum TestEnumWithGenericsAndFieldsAndLifetimes<'a, T: Clone>
where
    T: reflectapi::Input,
{
    Variant1(u8),
    Variant2(T, T),
    Variant3(std::borrow::Cow<'a, T>),
}

fn main() {}
