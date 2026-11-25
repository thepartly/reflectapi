//! `reflectapi::conventions::lints`: each lint against a route that breaks
//! its rule, and an API that follows them all.

// The types exist for their schemas: their fields are never read.
#![allow(dead_code)]

use std::collections::BTreeSet;

use reflectapi::conventions::lints;
use reflectapi::conventions::{ForwardPaginatedList, PageRequest};

// An API that follows every rule.

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
pub struct Product {
    pub id: String,
    pub unit_price: u32,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct ProductsSearchRequest {
    #[serde(default)]
    pub page: PageRequest,
    pub filter_by: ProductsSearchFilterBy,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct ProductsSearchFilterBy {
    pub status: Option<ProductStatus>,
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    OnSale,
    Withdrawn,
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProductsSearchError {
    #[reflectapi(status = 422)]
    InvalidCursor,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct ProductsGetRequest {
    pub id: String,
}

#[derive(serde::Serialize, reflectapi::Output)]
pub struct ProductsGetResponse {
    pub product: Product,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct Headers {
    pub authorization: String,
    #[serde(rename = "acme-store-key")]
    pub store_key: Option<String>,
}

async fn products_search(
    _: (),
    _: ProductsSearchRequest,
    _: Headers,
) -> Result<ForwardPaginatedList<Product>, ProductsSearchError> {
    unimplemented!()
}

async fn products_get(_: (), _: ProductsGetRequest, _: Headers) -> ProductsGetResponse {
    unimplemented!()
}

/// `Product` is written by two calls: it isn't either's own.
async fn products_insert(_: (), _: Product, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

async fn products_update(_: (), _: Product, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

fn conforming() -> reflectapi::Builder<()> {
    reflectapi::Builder::new()
        .route(products_search, |b| b.name("products.search"))
        .route(products_get, |b| b.name("products.get"))
        .route(products_insert, |b| b.name("products.insert"))
        .route(products_update, |b| b.name("products.update"))
}

#[test]
fn an_api_that_follows_the_rules_passes_every_lint() {
    let (schema, _) = conforming().build().unwrap();
    let errors: Vec<String> = lints::all(&schema).iter().map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn every_lint_can_be_given_to_the_builder() {
    let mut builder = conforming();
    for lint in lints::ALL {
        builder = builder.validate(*lint);
    }
    builder.build().unwrap();
}

// Routes that break one rule each, and the rules they break.

/// The rules `lint` reports for the routes `routes` adds, as
/// `(pointer, rule)`.
fn report(
    lint: lints::Lint,
    routes: impl FnOnce(reflectapi::Builder<()>) -> reflectapi::Builder<()>,
) -> BTreeSet<(String, String)> {
    let (schema, _) = routes(reflectapi::Builder::new()).build().unwrap();
    lint(&schema)
        .into_iter()
        .map(|e| {
            assert!(
                e.message
                    .contains("https://reflectapi.rs/design/standard.html#api-"),
                "{e}"
            );
            (e.pointer.to_string(), e.rule.unwrap())
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    expected
        .iter()
        .map(|(pointer, rule)| (pointer.to_string(), rule.to_string()))
        .collect()
}

async fn empty(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> reflectapi::Empty {
    reflectapi::Empty {}
}

#[test]
fn call_names_have_a_resource_and_an_action() {
    let reported = report(lints::call_name_form, |b| {
        b.route(empty, |b| b.name("health"))
            .route(empty, |b| b.name("pets..get"))
            .route(empty, |b| b.name("pets.get"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("health", "api-call-name-form"),
            ("pets..get", "api-call-name-form"),
        ])
    );
}

#[test]
fn call_names_are_kebab_case() {
    let reported = report(lints::call_names_kebab_case, |b| {
        b.route(empty, |b| b.name("productGroups.list"))
            .route(empty, |b| b.name("product_groups.list"))
            .route(empty, |b| b.name("product-groups.get-many"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("productGroups.list", "api-call-name-kebab-case"),
            ("product_groups.list", "api-call-name-kebab-case"),
        ])
    );
}

#[test]
fn call_names_have_two_levels_of_resources_at_most() {
    let reported = report(lints::call_resource_levels, |b| {
        b.route(empty, |b| b.name("billing.invoices.attachments.list"))
            .route(empty, |b| b.name("billing.invoices.attachments.pages.list"))
    });
    assert_eq!(
        reported,
        pairs(&[(
            "billing.invoices.attachments.pages.list",
            "api-resource-two-levels"
        )])
    );
}

#[test]
fn a_version_is_a_suffix_of_the_action() {
    let reported = report(lints::call_version_suffix, |b| {
        b.route(empty, |b| b.name("commerce.groups.v2.insert"))
            .route(empty, |b| b.name("commerce.groups.insert-v2"))
    });
    assert_eq!(
        reported,
        pairs(&[("commerce.groups.v2.insert", "api-call-version-suffix")])
    );

    let reported = report(lints::call_resource_levels, |b| {
        b.route(empty, |b| b.name("commerce.supplier.groups.v2.insert"))
    });
    assert!(reported.is_empty(), "{reported:?}");
}

#[test]
fn actions_a_standard_one_replaces_are_refused() {
    let reported = report(lints::call_actions_standard, |b| {
        b.route(empty, |b| b.name("pets.create"))
            .route(empty, |b| b.name("pets.delete-many"))
            .route(empty, |b| b.name("pets.upsert-v2"))
            .route(empty, |b| b.name("pets.fetch-experimental"))
            .route(empty, |b| b.name("pets.insert"))
            .route(empty, |b| b.name("orders.confirm"))
            .route(empty, |b| b.name("users.invite-v3"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("pets.create", "api-insert-not-create"),
            ("pets.delete-many", "api-remove-not-delete"),
            ("pets.upsert-v2", "api-action-standard"),
            ("pets.fetch-experimental", "api-action-standard"),
        ])
    );
}

#[test]
fn actions_dont_name_what_they_act_on() {
    let reported = report(lints::call_actions_not_parameters, |b| {
        b.route(empty, |b| b.name("products.search-by-name"))
            .route(empty, |b| b.name("users.update-password"))
            .route(empty, |b| b.name("products.find-by-sku"))
            .route(empty, |b| b.name("products.get-many"))
            .route(empty, |b| b.name("products.search-v2"))
            .route(empty, |b| b.name("users.password.update"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("products.search-by-name", "api-action-not-parameters"),
            ("users.update-password", "api-action-not-parameters"),
            ("products.find-by-sku", "api-action-not-parameters"),
        ])
    );
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct Id(pub String);

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(transparent)]
pub struct Ids {
    pub ids: Vec<String>,
}

async fn takes_a_string(_: (), _: String, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

async fn takes_a_newtype(_: (), _: Id, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

async fn takes_a_transparent_list(_: (), _: Ids, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

async fn takes_nothing(_: (), _: (), _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

#[test]
fn a_request_is_an_object() {
    let reported = report(lints::request_is_object, |b| {
        b.route(takes_nothing, |b| b.name("caches.reset"))
            .route(takes_a_string, |b| b.name("pets.get"))
            .route(takes_a_newtype, |b| b.name("pets.remove"))
            .route(takes_a_transparent_list, |b| b.name("pets.get-many"))
            .route(empty, |b| b.name("pets.list"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("pets.get", "api-arguments-in-body-object"),
            ("pets.remove", "api-arguments-in-body-object"),
            ("pets.get-many", "api-arguments-in-body-object"),
        ])
    );
}

async fn returns_a_list(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> Vec<Product> {
    unimplemented!()
}

async fn returns_an_option(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Option<ProductsGetResponse> {
    unimplemented!()
}

async fn returns_a_map(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> std::collections::HashMap<String, Product> {
    unimplemented!()
}

async fn returns_a_page(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> ForwardPaginatedList<Product> {
    unimplemented!()
}

/// Its case without data is a string, `"not_found"`.
#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(rename_all = "snake_case")]
pub enum PetsRemoveError {
    #[reflectapi(status = 404)]
    NotFound,
}

async fn fails_with_a_string(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, PetsRemoveError> {
    unimplemented!()
}

async fn fails_with_an_object(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, ProductsSearchError> {
    unimplemented!()
}

#[test]
fn a_response_is_an_object() {
    let reported = report(lints::response_is_object, |b| {
        b.route(returns_a_list, |b| b.name("products.list"))
            .route(returns_an_option, |b| b.name("products.get"))
            .route(returns_a_map, |b| b.name("products.lookup"))
            .route(returns_a_page, |b| b.name("products.search"))
            .route(fails_with_a_string, |b| b.name("pets.remove"))
            .route(fails_with_an_object, |b| b.name("pets.update"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("products.list", "api-response-is-object"),
            ("products.get", "api-response-is-object"),
            ("pets.remove", "api-response-is-object"),
        ])
    );
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct PetsInsertRequest {
    #[serde(rename = "petName")]
    pub pet_name: String,
    pub kind: PetKind,
    pub shape: PetShape,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub enum PetKind {
    Dog,
    #[serde(rename = "cat")]
    Cat,
    /// A code from an outside standard keeps its form.
    #[serde(rename = "NZ1")]
    NewZealandBreed,
}

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(tag = "Kind", rename_all = "snake_case")]
pub enum PetShape {
    Round {
        #[serde(rename = "Radius")]
        radius: u32,
    },
}

async fn pets_insert(_: (), _: PetsInsertRequest, _: Headers) -> reflectapi::Empty {
    unimplemented!()
}

#[test]
fn field_names_and_enum_values_are_snake_case() {
    let reported = report(lints::names_snake_case, |b| {
        b.route(pets_insert, |b| b.name("pets.insert"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("lints::PetsInsertRequest.petName", "api-name-snake-case"),
            ("lints::PetKind::Dog", "api-name-snake-case"),
            ("lints::PetShape", "api-name-snake-case"),
            ("lints::PetShape.round.Radius", "api-name-snake-case"),
        ])
    );
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct BadHeaders {
    #[serde(rename = "x-store-key")]
    pub store_key: String,
    #[serde(rename = "Store_Region")]
    pub store_region: String,
    #[serde(rename = "x-forwarded-for")]
    pub forwarded_for: Option<String>,
}

async fn with_bad_headers(_: (), _: reflectapi::Empty, _: BadHeaders) -> reflectapi::Empty {
    unimplemented!()
}

#[test]
fn header_names_are_kebab_case() {
    let reported = report(lints::header_names_kebab_case, |b| {
        b.route(with_bad_headers, |b| b.name("pets.list"))
    });
    assert_eq!(
        reported,
        pairs(&[("lints::BadHeaders.Store_Region", "api-header-name-case")])
    );
}

#[test]
fn own_headers_dont_start_with_x() {
    let reported = report(lints::header_names_no_x_prefix, |b| {
        b.route(with_bad_headers, |b| b.name("pets.list"))
    });
    assert_eq!(
        reported,
        pairs(&[("lints::BadHeaders.x-store-key", "api-header-own-prefix")])
    );
}

/// Named as `pets.list`'s request, used by `pets.search` alone.
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct PetsListRequest {
    pub name: String,
}

async fn pets_search_misnamed(
    _: (),
    _: PetsListRequest,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

mod pets_search {
    /// Named after the action, in a module of the resource's.
    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct SearchRequest {
        pub name: String,
    }
}

async fn pets_search_in_module(
    _: (),
    _: pets_search::SearchRequest,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

/// Not named as a call's: a type the call takes as it is.
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct NewPet {
    pub name: String,
}

async fn pets_insert_new_pet(_: (), _: NewPet, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct UsersSignInRequest {
    pub email: String,
}

async fn users_signin(_: (), _: UsersSignInRequest, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct OfferRepairerSearchRequest {
    pub query: String,
}

async fn offers_repairer_search(
    _: (),
    _: OfferRepairerSearchRequest,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct UsersInviteRequest {
    pub email: String,
}

async fn users_invite(_: (), _: UsersInviteRequest, _: reflectapi::Empty) -> reflectapi::Empty {
    unimplemented!()
}

#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(rename_all = "snake_case")]
pub enum PetsGetError {
    #[reflectapi(status = 404)]
    NotFoundError,
    #[reflectapi(status = 403)]
    Forbidden,
}

#[test]
fn a_type_named_as_a_calls_names_its_call() {
    let reported = report(lints::call_type_names, |b| {
        b.route(pets_search_misnamed, |b| b.name("pets.search"))
            .route(pets_search_in_module, |b| b.name("cats.search"))
            .route(pets_insert_new_pet, |b| b.name("pets.insert"))
            .route(users_signin, |b| b.name("users.signin"))
            .route(offers_repairer_search, |b| b.name("offers.repairer.search"))
            .route(users_invite, |b| b.name("users.invite-v2"))
            .route(products_search, |b| b.name("products.search"))
            .route(products_get, |b| b.name("products.get"))
            .route(products_insert, |b| b.name("products.insert"))
            .route(products_update, |b| b.name("products.update"))
    });
    assert_eq!(
        reported,
        pairs(&[("pets.search", "api-type-named-after-call")])
    );

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(pets_search_misnamed, |b| b.name("pets.search"))
        .build()
        .unwrap();
    let message = &lints::call_type_names(&schema)[0].message;
    assert!(message.contains("name it `PetsSearchRequest`"), "{message}");
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct PetStruct {
    pub name: String,
}

async fn pets_get(_: (), _: PetStruct, _: reflectapi::Empty) -> Result<Product, PetsGetError> {
    unimplemented!()
}

#[test]
fn type_names_dont_say_their_kind() {
    let reported = report(lints::type_names_no_kind_suffix, |b| {
        b.route(pets_get, |b| b.name("pets.get"))
    });
    assert_eq!(
        reported,
        pairs(&[("lints::PetStruct", "api-type-no-kind-suffix")])
    );
}

#[test]
fn error_cases_dont_end_with_error() {
    let reported = report(lints::error_cases_no_error_suffix, |b| {
        b.route(pets_get, |b| b.name("pets.get"))
    });
    assert_eq!(
        reported,
        pairs(&[(
            "lints::PetsGetError::not_found_error",
            "api-error-case-no-error-suffix"
        )])
    );
}

// How enums are written.

/// Cases with fields, written in serde's default form: `{"reschedule": {…}}`.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(rename_all = "snake_case")]
pub enum OrderChange {
    Reschedule { date: String },
    Cancel,
}

/// Cases with fields, written in a field named `type`.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderNote {
    Text { text: String },
}

/// Cases with fields, written in `kind`.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrderMove {
    Reassign { courier_id: String },
    Hold,
}

/// One value in each case: `{"sku": "A-1"}`.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(rename_all = "snake_case")]
pub enum OrderIdentity {
    Id(String),
    Number(u32),
}

/// No data: plain strings.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(rename_all = "snake_case")]
pub enum OrderPriority {
    Low,
    High,
}

/// An ingest item's action names its case.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum OrderItem {
    Insert { name: String },
    Remove { id: String },
}

/// Forms told apart by their fields.
#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(untagged)]
pub enum OrderLocator {
    ById { id: String },
    ByNumber { number: u32 },
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct OrdersUpdateRequest {
    pub change: OrderChange,
    pub note: OrderNote,
    pub moved: OrderMove,
    pub identity: OrderIdentity,
    pub priority: OrderPriority,
    pub items: Vec<OrderItem>,
    pub locator: OrderLocator,
}

async fn orders_update(
    _: (),
    _: OrdersUpdateRequest,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, PetsRemoveError> {
    unimplemented!()
}

#[test]
fn enums_with_fields_and_errors_are_tagged_by_kind() {
    let reported = report(lints::enums_kind_tag, |b| {
        b.route(orders_update, |b| b.name("orders.update"))
            .route(fails_with_an_object, |b| b.name("pets.update"))
    });
    assert_eq!(
        reported,
        pairs(&[
            ("lints::OrderChange", "api-enum-kind-tag"),
            ("lints::OrderNote", "api-enum-kind-tag"),
            ("lints::PetsRemoveError", "api-enum-kind-tag"),
        ])
    );
}

// Type paths. The types of this file are under `lints::`, one module.

mod billing {
    pub mod pricing {
        /// `lints::billing::pricing::BrandRate`: three modules.
        #[derive(serde::Deserialize, reflectapi::Input)]
        pub struct BrandRate {
            pub rate: u32,
        }
    }

    pub mod v2 {
        /// `lints::billing::v2::Baseline`: two modules, a version not counted.
        #[derive(serde::Deserialize, reflectapi::Input)]
        pub struct Baseline {
            pub rate: u32,
        }
    }
}

mod models {
    /// `lints::models::Conversation`: a module that names how code is organised.
    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Conversation {
        pub id: String,
    }
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub struct BillingPricingUpdateRequest {
    pub rate: billing::pricing::BrandRate,
    pub baseline: billing::v2::Baseline,
    pub conversation: models::Conversation,
}

async fn billing_pricing_update(
    _: (),
    _: BillingPricingUpdateRequest,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

#[test]
fn type_paths_have_two_modules_at_most() {
    let reported = report(lints::type_path_depth, |b| {
        b.route(billing_pricing_update, |b| b.name("billing.pricing.update"))
    });
    assert_eq!(
        reported,
        pairs(&[("lints::billing::pricing::BrandRate", "api-type-path-depth")])
    );
}

#[test]
fn type_modules_dont_name_how_code_is_organised() {
    let reported = report(lints::type_modules_not_implementation, |b| {
        b.route(billing_pricing_update, |b| b.name("billing.pricing.update"))
    });
    assert_eq!(
        reported,
        pairs(&[(
            "lints::models::Conversation",
            "api-type-modules-not-implementation"
        )])
    );
}
