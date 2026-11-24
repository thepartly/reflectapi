//! TypeScript type overrides: a client writes a schema type as a TypeScript
//! type of its choosing, checked by `tsc` against the type's wire shape (#240).

use reflectapi::codegen::typescript::{generate, Config};

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct Money {
    amount: String,
    currency: String,
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct Order {
    id: uuid::Uuid,
    customer: Option<uuid::Uuid>,
    lines: Vec<uuid::Uuid>,
    placed: chrono::DateTime<chrono::Utc>,
    price: Money,
    count: u64,
}

const UUID: &str = "`${string}-${string}-${string}-${string}-${string}`";
const INSTANT: &str = r#"string & { readonly instant: unique symbol }"#;
const MONEY: &str = r#"Readonly<{ amount: `${number}`; currency: "NZD" | "AUD" }>"#;

fn schema() -> reflectapi::Schema {
    super::into_schema::<Order>()
}

fn typescript(config: &Config) -> anyhow::Result<String> {
    let files = generate(schema(), config)?;
    Ok(reflectapi::codegen::strip_boilerplate(
        &files["generated.ts"],
    ))
}

#[test]
fn overrides_replace_each_reference_and_are_checked_by_tsc() {
    let generated = typescript(
        Config::default()
            .format(true)
            .typecheck(true)
            .type_override("uuid::Uuid", UUID)
            .type_override("chrono::DateTime", INSTANT)
            .type_override("u64", "`${bigint}`")
            .type_override("reflectapi_demo::tests::type_overrides::Money", MONEY),
    );
    let error = generated.unwrap_err().to_string();
    // `u64` is a number on the wire, so a string override is refused.
    assert!(
        error.contains("does not satisfy the constraint 'number'"),
        "{error}"
    );

    let generated = typescript(
        Config::default()
            .format(true)
            .typecheck(true)
            .type_override("uuid::Uuid", UUID)
            .type_override("chrono::DateTime", INSTANT)
            .type_override("reflectapi_demo::tests::type_overrides::Money", MONEY),
    )
    .unwrap();
    insta::assert_snapshot!(generated);
}

#[test]
fn an_override_sent_as_something_else_fails_tsc() {
    let error = typescript(
        Config::default()
            .typecheck(true)
            .type_override("chrono::DateTime", "Date"),
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Type 'Date' does not satisfy the constraint 'string'"),
        "{error}"
    );
}

#[test]
fn an_override_of_a_type_the_schema_lacks_is_refused() {
    let error = typescript(Config::default().type_override("uuid::Uid", "string"))
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "type override for `uuid::Uid`: the schema has no type `uuid::Uid`"
    );
}

#[test]
fn an_override_of_a_generic_type_is_refused() {
    let error = typescript(Config::default().type_override("std::vec::Vec", "string[]"))
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "type override for `std::vec::Vec`: a type with type parameters can't be overridden"
    );
}

/// `Order` differs between requests and responses: a request may leave
/// `customer` out. The builder generates it as two types, `input::Order` and
/// `output::Order`, and an override named by the Rust type replaces both.
#[test]
fn an_override_of_a_type_split_into_input_and_output_replaces_both() {
    const ORDER: &str = "reflectapi_demo::tests::type_overrides::Order";
    let files = generate(
        schema(),
        Config::default().typecheck(true).type_override(
            ORDER,
            "Readonly<reflectapi_demo.tests.type_overrides.output.Order>",
        ),
    )
    .unwrap();
    let generated = &files["generated.ts"];
    for split in ["input", "output"] {
        let check = format!(
            "\"reflectapi_demo::tests::type_overrides::{split}::Order\": \
             __TypeOverride<__wire.reflectapi_demo.tests.type_overrides.{split}.Order, \
             Readonly<reflectapi_demo.tests.type_overrides.output.Order>>;"
        );
        assert!(generated.contains(&check), "{generated}");
    }

    // An input `Order` may lack `customer`, so it isn't an output `Order`.
    let error = generate(
        schema(),
        Config::default()
            .typecheck(true)
            .type_override(ORDER, "reflectapi_demo.tests.type_overrides.input.Order"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("does not satisfy the constraint"), "{error}");
    assert!(
        error.contains("reflectapi_demo.tests.type_overrides.output.Order'."),
        "{error}"
    );
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct Line {
    product: uuid::Uuid,
    quantity: u32,
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct Basket {
    lines: Vec<Line>,
}

/// Each override is checked against its type as it is on the wire, not as
/// other overrides have narrowed it: `Line`'s override has a plain string
/// `product`, which a branded `uuid::Uuid` override doesn't change.
#[test]
fn an_override_is_checked_against_its_type_without_other_overrides() {
    let files = generate(
        super::into_schema::<Basket>(),
        Config::default()
            .format(true)
            .typecheck(true)
            .type_override("uuid::Uuid", "string & { readonly uuid: unique symbol }")
            .type_override(
                "reflectapi_demo::tests::type_overrides::Line",
                "{ product: string; quantity: number }",
            ),
    )
    .unwrap();
    insta::assert_snapshot!(reflectapi::codegen::strip_boilerplate(
        &files["generated.ts"]
    ));
}
