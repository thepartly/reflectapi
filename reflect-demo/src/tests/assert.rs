pub async fn handler<I, O>(_state: (), _input: I, _headers: reflect::Empty) -> O
where
    I: reflect::Input,
    O: reflect::Output,
{
    unimplemented!() // should be never called
}

pub fn into_input_schema<I>() -> reflect::Schema
where
    I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
{
    let eps = reflect::Builder::new()
        .route(handler::<I, reflect::Empty>, |b| {
            b.name("input_test".into())
        })
        .build(Default::default(), Default::default())
        .unwrap();
    eps.0
}

pub fn into_output_schema<O>() -> reflect::Schema
where
    O: reflect::Output + serde::ser::Serialize + Send + 'static,
{
    let eps = reflect::Builder::new()
        .route(handler::<reflect::Empty, O>, |b| {
            b.name("output_test".into())
        })
        .build(Default::default(), Default::default())
        .unwrap();
    eps.0
}

pub fn into_schema<T>() -> reflect::Schema
where
    T: reflect::Input
        + serde::de::DeserializeOwned
        + reflect::Output
        + serde::ser::Serialize
        + Send
        + 'static,
{
    let mut eps = reflect::Builder::new()
        .route(handler::<T, T>, |b| b)
        .build(Default::default(), Default::default())
        .unwrap();
    eps.0.functions.clear(); // remove the function signature from snapshots
    eps.0
}

pub fn into_input_typescript_code<I>() -> String
where
    I: reflect::Input + serde::de::DeserializeOwned + Send + 'static,
{
    let eps = into_input_schema::<I>();
    reflect::codegen::typescript::generate(eps).unwrap()
}

pub fn into_output_typescript_code<O>() -> String
where
    O: reflect::Output + serde::ser::Serialize + Send + 'static,
{
    let eps = into_output_schema::<O>();
    reflect::codegen::typescript::generate(eps).unwrap()
}

pub fn into_typescript_code<T>() -> String
where
    T: reflect::Input
        + serde::de::DeserializeOwned
        + reflect::Output
        + serde::ser::Serialize
        + Send
        + 'static,
{
    let eps = into_schema::<T>();
    reflect::codegen::typescript::generate(eps).unwrap()
}

macro_rules! assert_input_snapshot {
    ($I:ty) => {
        insta::assert_json_snapshot!(super::into_input_schema::<$I>().input_types);
        insta::assert_snapshot!(super::into_input_typescript_code::<$I>());
    };
}

macro_rules! assert_output_snapshot {
    ($O:ty) => {
        insta::assert_json_snapshot!(super::into_output_schema::<$O>().output_types);
        insta::assert_snapshot!(super::into_output_typescript_code::<$O>());
    };
}

macro_rules! assert_snapshot {
    ($T:ty) => {
        insta::assert_json_snapshot!(super::into_schema::<$T>());
        insta::assert_snapshot!(super::into_typescript_code::<$T>());
    };
}
