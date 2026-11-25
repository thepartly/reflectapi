//! Type parameters a schema doesn't use are dropped, with their arguments.

use reflectapi::Encoded;

/// What a cursor holds: only the server reads it.
#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
struct PetsCursor {
    after_id: u64,
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
#[serde(bound(deserialize = "C: serde::de::DeserializeOwned"))]
struct Pagination<C> {
    next_cursor: Option<Encoded<C>>,
}

/// `C` reaches `Pagination` only, so it goes from here once it goes there.
#[derive(serde::Serialize, reflectapi::Output)]
struct Page<D, C> {
    items: Vec<D>,
    pagination: Option<Pagination<C>>,
}

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(bound(deserialize = "C: serde::de::DeserializeOwned"))]
struct PageRequest<C> {
    #[allow(dead_code)]
    cursor: Option<Encoded<C>>,
}

async fn pets_list(
    _: (),
    _: PageRequest<PetsCursor>,
    _: reflectapi::Empty,
) -> Page<u32, PetsCursor> {
    unimplemented!()
}

fn builder() -> reflectapi::Builder<()> {
    reflectapi::Builder::new().route(pets_list, |b| b.name("pets.list"))
}

fn parameters(ty: &reflectapi::Type) -> Vec<String> {
    match ty {
        reflectapi::Type::Struct(s) => s.parameters().map(|p| p.name.clone()).collect(),
        reflectapi::Type::Enum(e) => e.parameters().map(|p| p.name.clone()).collect(),
        reflectapi::Type::Primitive(p) => p.parameters().map(|p| p.name.clone()).collect(),
    }
}

#[test]
fn a_parameter_reflection_erases_is_dropped() {
    let (schema, _) = builder().build().unwrap();
    let module = "reflectapi_demo::tests::type_parameters";

    let page = schema
        .output_types()
        .get_type(&format!("{module}::Page"))
        .unwrap();
    assert_eq!(parameters(page), ["D"]);
    let pagination = schema
        .output_types()
        .get_type(&format!("{module}::Pagination"))
        .unwrap();
    assert!(parameters(pagination).is_empty());

    let function = schema.functions().next().unwrap();
    let reflectapi::OutputType::Complete {
        output_type: Some(output),
    } = function.output_type()
    else {
        panic!("expected an output type");
    };
    let arguments: Vec<&str> = output.arguments().map(|a| a.name.as_str()).collect();
    assert_eq!(arguments, ["u32"]);
    assert!(function.input_type().unwrap().arguments.is_empty());

    // The cursor's type reached the schema only through those arguments.
    let cursor = format!("{module}::PetsCursor");
    assert!(schema.input_types().get_type(&cursor).is_none());
    assert!(schema.output_types().get_type(&cursor).is_none());
}

#[test]
fn test_unused_type_parameters_are_dropped() {
    assert_builder_snapshot!(builder())
}
