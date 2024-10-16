pub async fn handler<I, O>(_state: (), _input: I, _headers: reflectapi::Empty) -> O
where
    I: reflectapi::Input,
    O: reflectapi::Output,
{
    unimplemented!() // should be never called
}

fn mk_config() -> reflectapi::codegen::Config {
    reflectapi::codegen::Config {
        format: true,
        // Typechecking is too slow to run locally on every test
        typecheck: std::env::var("CI").is_ok(),
        ..Default::default()
    }
}

pub fn into_input_schema<I>() -> reflectapi::Schema
where
    I: reflectapi::Input + serde::de::DeserializeOwned + Send + 'static,
{
    let eps = reflectapi::Builder::new()
        .route(handler::<I, reflectapi::Empty>, |b| {
            b.name("input_test".into())
        })
        .fold_transparent_types()
        .build()
        .unwrap();
    eps.0
}

pub fn into_output_schema<O>() -> reflectapi::Schema
where
    O: reflectapi::Output + serde::ser::Serialize + Send + 'static,
{
    let eps = reflectapi::Builder::new()
        .route(handler::<reflectapi::Empty, O>, |b| {
            b.name("output_test".into())
        })
        .fold_transparent_types()
        .build()
        .unwrap();
    eps.0
}

pub fn into_schema<T>() -> reflectapi::Schema
where
    T: reflectapi::Input
        + serde::de::DeserializeOwned
        + reflectapi::Output
        + serde::ser::Serialize
        + Send
        + 'static,
{
    let eps = reflectapi::Builder::new()
        .route(handler::<T, T>, |b| b.name("inout_test".into()))
        .fold_transparent_types()
        .build()
        .unwrap();
    eps.0
}

fn codegen_rust(schema: reflectapi::Schema) -> String {
    reflectapi::codegen::strip_boilerplate(
        &reflectapi::codegen::rust::generate(schema, &mk_config()).unwrap(),
    )
}

fn codegen_typescript(schema: reflectapi::Schema) -> String {
    reflectapi::codegen::strip_boilerplate(
        &reflectapi::codegen::typescript::generate(schema, &mk_config()).unwrap(),
    )
}

pub fn into_input_typescript_code<I>() -> String
where
    I: reflectapi::Input + serde::de::DeserializeOwned + Send + 'static,
{
    let eps = into_input_schema::<I>();
    codegen_typescript(eps)
}

pub fn into_output_typescript_code<O>() -> String
where
    O: reflectapi::Output + serde::ser::Serialize + Send + 'static,
{
    let eps = into_output_schema::<O>();
    codegen_typescript(eps)
}

pub fn into_typescript_code<T>() -> String
where
    T: reflectapi::Input
        + serde::de::DeserializeOwned
        + reflectapi::Output
        + serde::ser::Serialize
        + Send
        + 'static,
{
    let eps = into_schema::<T>();
    codegen_typescript(eps)
}

pub fn into_input_rust_code<I>() -> String
where
    I: reflectapi::Input + serde::de::DeserializeOwned + Send + 'static,
{
    let eps = into_input_schema::<I>();
    codegen_rust(eps)
}

pub fn into_output_rust_code<O>() -> String
where
    O: reflectapi::Output + serde::ser::Serialize + Send + 'static,
{
    let eps = into_output_schema::<O>();
    codegen_rust(eps)
}

pub fn into_rust_code<T>() -> String
where
    T: reflectapi::Input
        + serde::de::DeserializeOwned
        + reflectapi::Output
        + serde::ser::Serialize
        + Send
        + 'static,
{
    let eps = into_schema::<T>();
    codegen_rust(eps)
}

macro_rules! assert_input_snapshot {
    ($I:ty) => {
        insta::assert_json_snapshot!(super::into_input_schema::<$I>().input_types);
        insta::assert_snapshot!(super::into_input_typescript_code::<$I>());
        insta::assert_snapshot!(super::into_input_rust_code::<$I>());
    };
}

macro_rules! assert_output_snapshot {
    ($O:ty) => {
        insta::assert_json_snapshot!(super::into_output_schema::<$O>().output_types);
        insta::assert_snapshot!(super::into_output_typescript_code::<$O>());
        insta::assert_snapshot!(super::into_output_rust_code::<$O>());
    };
}

macro_rules! assert_snapshot {
    ($T:ty) => {{
        let schema = super::into_schema::<$T>();
        insta::assert_json_snapshot!(schema);
        insta::assert_snapshot!(super::into_typescript_code::<$T>());
        insta::assert_snapshot!(super::into_rust_code::<$T>());
        insta::assert_json_snapshot!(reflectapi::codegen::openapi::Spec::from(&schema));
    }};
}

macro_rules! assert_builder_snapshot {
    ($builder:expr) => {{
        let (schema, _) = $builder.build().unwrap();
        let config = reflectapi::codegen::Config {
            format: true,
            typecheck: true,
            ..Default::default()
        };
        let rust = reflectapi::codegen::rust::generate(schema.clone(), &config).unwrap();
        let typescript =
            reflectapi::codegen::typescript::generate(schema.clone(), &config).unwrap();
        insta::assert_json_snapshot!(schema);
        insta::assert_snapshot!(typescript);
        insta::assert_snapshot!(rust);
        insta::assert_json_snapshot!(reflectapi::codegen::openapi::Spec::from(&schema));
    }};
}
