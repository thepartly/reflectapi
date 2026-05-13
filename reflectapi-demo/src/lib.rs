use std::sync::{Arc, Mutex};

use futures_util::Stream;
use tokio::sync::broadcast;

#[cfg(test)]
mod tests;

pub fn builder() -> reflectapi::Builder<Arc<AppState>> {
    // build an application, by providing pointers to handlers
    // and assigning those to named routes
    reflectapi::Builder::new()
        .name("Demo application")
        .description("This is a demo application")
        .route(health_check, |b| {
            b.name("health.check")
                .readonly(true)
                .tag("internal")
                .description("Check the health of the service")
        })
        .route(pets_list, |b| {
            b.name("pets.list")
                .readonly(true)
                .description("List available pets")
        })
        .route(pets_create, |b| {
            b.name("pets.create")
                .description("Create a new pet")
        })
        .route(pets_update, |b| {
            b.name("pets.update")
                .description("Update an existing pet")
        })
        .route(pets_remove, |b| {
            b.name("pets.remove")
                .description("Remove an existing pet")
        })
        .route(pets_remove, |b| {
            b.name("pets.delete")
                .description("Remove an existing pet")
                .deprecation_note("Use pets.remove instead")
        })
        .route(pets_get_first, |b| {
            b.name("pets.get-first")
                .description("Fetch first pet, if any exists")
        })
        .stream_route(pets_cdc_events, |b| {
            b.name("pets.cdc-events")
                .readonly(true)
                .description("Stream of change data capture events for pets")
        })
        .route(order_coverage, |b| {
            b.name("codegen-order-coverage")
                .description("Coverage fixtures for namespace/tuple/Duration/PhantomData rendering")
        })
        .route(codegen_coverage, |b| {
            b.name("codegen-coverage")
                .description("Coverage fixtures for codegen edge cases")
        })
        .rename_types("reflectapi_demo::", "myapi::")
        // and some optional linting rules
        .validate(|schema| {
            let mut errors = Vec::new();
            for f in schema.functions() {
                if f.name().chars().any(|c| !c.is_ascii_alphanumeric() && c != '.' && c != '-') {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Function(f.name().to_string()),
                        "Function names must contain only alphanumeric characters, dots and dashes only".into(),
                    ));
                }
            }
            errors
        })
}

// Test for struct error returns.
#[derive(Debug, serde::Serialize, reflectapi::Output)]
struct HealthCheckFail {}

impl reflectapi::StatusCode for HealthCheckFail {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn health_check(
    _: Arc<AppState>,
    _request: reflectapi::Empty,
    _headers: reflectapi::Empty,
) -> Result<reflectapi::Empty, HealthCheckFail> {
    Ok(reflectapi::Empty {})
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
pub struct OrderCoverageRequest {
    pub order: order::OrderInsertData,
    pub rate_limit: order::RateLimit,
    pub policy: order::Policy<String, u32>,
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
pub struct OrderCoverageResponse {
    pub ok: bool,
}

/// Endpoint that drags the `order::` coverage fixtures into the
/// schema so the smoke test exercises their rendering. Not intended
/// to be called from the demo server.
async fn order_coverage(
    _: Arc<AppState>,
    request: OrderCoverageRequest,
    _headers: reflectapi::Empty,
) -> Result<OrderCoverageResponse, reflectapi::Infallible> {
    let _ = request;
    Ok(OrderCoverageResponse { ok: true })
}

/// Endpoint that drags the `coverage::` fixtures into the schema.
async fn codegen_coverage(
    _: Arc<AppState>,
    request: coverage::CoverageRequest,
    _headers: reflectapi::Empty,
) -> Result<coverage::CoverageResponse, reflectapi::Infallible> {
    let _ = request;
    Ok(coverage::CoverageResponse { ok: true })
}

#[derive(Debug)]
pub struct AppState {
    pets: Mutex<Vec<model::Pet>>,
    tx: broadcast::Sender<model::Pet>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pets: Mutex::new(Vec::new()),
            tx: broadcast::channel(100).0,
        }
    }
}

// Coverage fixtures exercised by the codegen smoke tests.
mod order {
    use std::marker::PhantomData;
    use std::time::Duration;

    /// A struct whose name begins with the parent namespace cap.
    /// Exercises the namespace-alias path for both the stripped
    /// (`order.InsertData`) and Rust-leaf (`order.OrderInsertData`)
    /// forms.
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct OrderInsertData {
        pub identity: String,
        /// Exercises the `tuple[A, B]` rendering for `(A, B)`.
        pub alternative_part_number: Option<(String, String)>,
    }

    /// Exercises the `std::time::Duration` ↔ `{secs, nanos}` wire
    /// adapter.
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct RateLimit {
        pub retry_after: Duration,
        pub max_wait: Option<Duration>,
    }

    /// Exercises `PhantomData<T>` elision (the field carries no wire
    /// data and must not appear in the Python model).
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Policy<C, T>
    where
        C: 'static,
        T: 'static,
    {
        pub name: String,
        pub _context_marker: PhantomData<C>,
        pub _output_marker: PhantomData<T>,
    }
}

/// Coverage fixtures for codegen edge cases. Each type exercises a
/// specific rendering rule; the CI smoke test (regenerate +
/// `import api_client`) confirms every shape round-trips through
/// JSON cleanly.
mod coverage {
    use std::collections::HashMap;

    // ---- Python-keyword and builtin field names ----
    // serde serialises these names verbatim; the codegen must produce
    // a safe Python identifier and carry the wire name as a Field
    // alias so the round-trip is preserved.

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct PyKeywordFields {
        #[serde(rename = "class")]
        pub class_: String,
        #[serde(rename = "from")]
        pub from_: u32,
        #[serde(rename = "import")]
        pub import_: bool,
        #[serde(rename = "lambda")]
        pub lambda_: i64,
        #[serde(rename = "return")]
        pub return_: Vec<u8>,
        #[serde(rename = "yield")]
        pub yield_: Option<String>,
        #[serde(rename = "None")]
        pub none_: Option<i32>,
        #[serde(rename = "True")]
        pub true_: bool,
        // Python builtins that aren't keywords but shadow ergonomics.
        pub r#type: String,
        pub r#match: String,
    }

    // ---- Pydantic-reserved field names ----
    // These names collide with `BaseModel`'s own attributes/methods on
    // the Python side; the generated class must rename and alias them.
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct PydanticReservedFields {
        #[serde(rename = "model_config")]
        pub model_config_: String,
        #[serde(rename = "model_fields_set")]
        pub model_fields_set_: String,
        #[serde(rename = "model_dump_json")]
        pub model_dump_json_: String,
    }

    // ---- Self-referential type ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct TreeNode {
        pub value: String,
        pub children: Vec<TreeNode>,
        pub parent: Option<Box<TreeNode>>,
    }

    // ---- Mutually recursive types ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct MutualA {
        pub name: String,
        pub b: Option<Box<MutualB>>,
    }
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct MutualB {
        pub name: String,
        pub a: Option<Box<MutualA>>,
    }

    // ---- Generic recursive type ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct GenericTree<T> {
        pub value: T,
        pub children: Vec<GenericTree<T>>,
    }

    // ---- Enum with variants whose names are Python keywords ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum KeywordVariants {
        Class,
        Lambda { name: String },
        Return { value: i32 },
    }

    // ---- HashMap with non-string keys (JSON stringifies them) ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct IntKeyedMap {
        pub by_id: HashMap<u64, String>,
        pub uuid_keyed: HashMap<String, String>,
    }

    // ---- Transparent newtype (no Python class emitted) ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(transparent)]
    pub struct UserId(pub u64);

    // ---- Three-state Option wrapping a regular Option ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct DeepOption {
        pub maybe_maybe: reflectapi::Option<Option<String>>,
    }

    // ---- Field names shared with symbols imported into the
    //      generated module (`BaseModel`, `Field`, `Annotated`, etc.) ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct ShadowingFields {
        pub field: String,
        pub annotated: String,
        pub generic: String,
        pub base_model: String,
    }

    // ---- Empty struct ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct EmptyStruct {}

    // ---- Docstring with characters that need escaping ----
    /// A docstring with "quotes" and 'apostrophes' and a backslash: \\
    /// And a """triple quote""" inside.
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct WeirdDocstring {
        pub value: String,
        /// Field description with "double quotes" and 'single quotes'.
        pub mixed_quotes: String,
        /// Field description with only "double quotes" — should use single-quoted Python literal.
        pub doubles_only: String,
    }

    // ---- Type name that shadows a Pydantic-imported symbol ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct BaseModel {
        pub label: String,
    }

    // ---- Generic used with multiple concrete arguments ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Wrapper<T> {
        pub inner: T,
    }

    // ---- Field with a non-None serde default ----
    fn default_count() -> u32 {
        42
    }
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct DefaultedField {
        #[serde(default = "default_count")]
        pub count: u32,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct CoverageRequest {
        pub keywords: PyKeywordFields,
        pub reserved: PydanticReservedFields,
        pub tree: TreeNode,
        pub mutual: MutualA,
        pub generic_tree: GenericTree<i32>,
        pub keyword_variants: KeywordVariants,
        pub int_keyed: IntKeyedMap,
        pub user_id: UserId,
        pub deep_option: DeepOption,
        pub shadowing: ShadowingFields,
        pub empty: EmptyStruct,
        pub weird_doc: WeirdDocstring,
        pub shadow_base_model: BaseModel,
        pub wrapper_int: Wrapper<i32>,
        pub wrapper_str: Wrapper<String>,
        pub defaulted: DefaultedField,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct CoverageResponse {
        pub ok: bool,
    }
}

mod model {
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Pet {
        /// identity
        pub name: String,
        /// kind of pet
        pub kind: Kind,
        /// age of the pet
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[deprecated(note = "test deprecation")]
        pub age: Option<u8>,
        #[serde(default = "super::some_datetime")]
        pub updated_at: chrono::DateTime<chrono::Utc>,
        /// behaviors of the pet
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub behaviors: Vec<Behavior>,
    }

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum Kind {
        /// A dog
        Dog {
            /// breed of the dog
            breed: String,
        },
        /// A cat
        Cat {
            /// lives left
            lives: u8,
        },
        /// Test for unit variants in internally tagged enums
        Bird,
    }

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub enum Behavior {
        Calm,
        Aggressive(/** aggressiveness level */ f64, /** some notes */ String),
        Other {
            /// Custom provided description of a behavior
            description: String,
            /// Additional notes
            /// Up to a user to put free text here
            #[serde(default, skip_serializing_if = "String::is_empty")]
            notes: String,
        },
    }
}

fn authorize<E: From<proto::UnauthorizedError>>(headers: proto::Headers) -> Result<(), E> {
    if headers.authorization.is_empty() {
        return Err(E::from(proto::UnauthorizedError));
    }
    Ok(())
}

async fn pets_list(
    state: Arc<AppState>,
    request: proto::PetsListRequest,
    headers: proto::Headers,
) -> Result<proto::Paginated<model::Pet>, proto::PetsListError> {
    authorize::<proto::PetsListError>(headers)?;

    let pets = state.pets.lock().unwrap();
    let cursor = request
        .cursor
        .unwrap_or("0".into())
        .parse()
        .map_err(|_| proto::PetsListError::InvalidCursor)?;
    let limit = request.limit.unwrap_or(u8::MAX) as usize;
    let result_items = pets
        .iter()
        .skip(cursor)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let result_cursor = if result_items.is_empty() {
        None
    } else {
        Some((cursor + limit).to_string())
    };
    Ok(proto::Paginated {
        items: result_items,
        cursor: result_cursor,
    })
}

async fn pets_create(
    state: Arc<AppState>,
    request: proto::PetsCreateRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsCreateError> {
    authorize::<proto::PetsCreateError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    if request.0.name.is_empty() {
        return Err(proto::PetsCreateError::InvalidIdentity {
            message: "Name is required".into(),
        });
    }

    if pets.iter().any(|pet| pet.name == request.0.name) {
        return Err(proto::PetsCreateError::Conflict);
    }

    state.tx.send(request.0.clone()).ok();
    pets.push(request.0);

    Ok(().into())
}

async fn pets_update(
    state: Arc<AppState>,
    request: proto::PetsUpdateRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsUpdateError> {
    authorize::<proto::PetsUpdateError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    let Some(pos) = pets.iter().position(|pet| pet.name == request.name) else {
        return Err(proto::PetsUpdateError::NotFound);
    };
    let pet = &mut pets[pos];

    if let Some(kind) = request.kind {
        pet.kind = kind;
    }
    #[allow(deprecated)]
    if let Some(age) = request.age.unfold() {
        pet.age = age.cloned();
    }

    if let Some(behaviors) = request.behaviors.unfold() {
        pet.behaviors = behaviors.cloned().unwrap_or_default();
    }

    pet.updated_at = some_datetime();
    state.tx.send(pet.clone()).ok();

    Ok(().into())
}

async fn pets_remove(
    state: Arc<AppState>,
    request: proto::PetsRemoveRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsRemoveError> {
    authorize::<proto::PetsRemoveError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    let Some(pos) = pets.iter().position(|pet| pet.name == request.name) else {
        return Err(proto::PetsRemoveError::NotFound);
    };
    state.tx.send(pets[pos].clone()).ok();
    pets.remove(pos);

    Ok(().into())
}

fn some_datetime() -> chrono::DateTime<chrono::Utc> {
    // Some fixed date for the sake of example
    chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00.000000000Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

async fn pets_get_first(
    state: Arc<AppState>,
    _: reflectapi::Empty,
    headers: proto::Headers,
) -> Result<Option<model::Pet>, proto::UnauthorizedError> {
    authorize::<proto::UnauthorizedError>(headers)?;

    let pets = state.pets.lock().unwrap();
    let random_pet = pets.first().cloned();

    Ok(random_pet)
}

fn pets_cdc_events(
    state: Arc<AppState>,
    _: reflectapi::Empty,
    headers: proto::Headers,
) -> Result<impl Stream<Item = model::Pet>, proto::UnauthorizedError> {
    authorize::<proto::UnauthorizedError>(headers)?;

    let mut rx = state.tx.subscribe();
    Ok(async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(pet) => yield pet,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

mod proto {
    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct InternalError {
        pub message: String,
    }

    impl reflectapi::StatusCode for InternalError {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    #[derive(Debug, serde::Serialize, reflectapi::Output)]
    pub struct UnauthorizedError;

    impl reflectapi::StatusCode for UnauthorizedError {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::UNAUTHORIZED
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Headers {
        /// Authorization header
        pub authorization: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Paginated<T>
    where
        T: reflectapi::Output,
    {
        /// slice of a collection
        pub items: Vec<T>,
        /// cursor for getting next page
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cursor: Option<String>,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsListRequest {
        #[serde(default)]
        pub limit: Option<u8>,
        #[serde(default)]
        pub cursor: Option<String>,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    #[serde(tag = "kind")]
    pub enum PetsListError {
        InvalidCursor,
        Unauthorized,
        #[allow(dead_code)]
        Internal(InternalError),
    }

    impl reflectapi::StatusCode for PetsListError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsListError::InvalidCursor => http::StatusCode::BAD_REQUEST,
                PetsListError::Unauthorized => http::StatusCode::UNAUTHORIZED,
                PetsListError::Internal(err) => err.status_code(),
            }
        }
    }

    impl From<UnauthorizedError> for PetsListError {
        fn from(_: UnauthorizedError) -> Self {
            PetsListError::Unauthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsCreateRequest(pub crate::model::Pet);

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsCreateError {
        Conflict,
        NotAuthorized,
        InvalidIdentity { message: String },
    }

    impl reflectapi::StatusCode for PetsCreateError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsCreateError::Conflict => http::StatusCode::CONFLICT,
                PetsCreateError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
                PetsCreateError::InvalidIdentity { .. } => http::StatusCode::UNPROCESSABLE_ENTITY,
            }
        }
    }

    impl From<UnauthorizedError> for PetsCreateError {
        fn from(_: UnauthorizedError) -> Self {
            PetsCreateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsUpdateRequest {
        /// identity
        pub name: String,
        /// kind of pet, non nullable in the model
        #[serde(default, skip_serializing_if = "Option::is_undefined")]
        pub kind: Option<crate::model::Kind>,
        /// age of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub age: reflectapi::Option<u8>,
        /// behaviors of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub behaviors: reflectapi::Option<Vec<crate::model::Behavior>>,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct ValidationA {
        pub message: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum ValidationError {
        #[allow(dead_code)]
        ValidationA(ValidationA),
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsUpdateError {
        NotFound,
        NotAuthorized,
        #[allow(dead_code)]
        Validation(Vec<ValidationError>),
    }

    impl reflectapi::StatusCode for PetsUpdateError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsUpdateError::NotFound => http::StatusCode::NOT_FOUND,
                PetsUpdateError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
                PetsUpdateError::Validation(_) => http::StatusCode::UNPROCESSABLE_ENTITY,
            }
        }
    }

    impl From<UnauthorizedError> for PetsUpdateError {
        fn from(_: UnauthorizedError) -> Self {
            PetsUpdateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsRemoveRequest {
        /// identity
        pub name: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsRemoveError {
        NotFound,
        NotAuthorized,
    }

    impl reflectapi::StatusCode for PetsRemoveError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsRemoveError::NotFound => http::StatusCode::NOT_FOUND,
                PetsRemoveError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
            }
        }
    }

    impl From<UnauthorizedError> for PetsRemoveError {
        fn from(_: UnauthorizedError) -> Self {
            PetsRemoveError::NotAuthorized
        }
    }
}
