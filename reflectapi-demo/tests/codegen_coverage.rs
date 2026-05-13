//! Codegen coverage fixtures + smoke test.
//!
//! Every type in this file exercises a specific Python codegen
//! rendering path. They live here as an integration test (rather
//! than in the demo's public schema) so the demo stays a clean
//! example and the fixtures stay clearly test-only.
//!
//! The `#[test]` at the bottom builds a private schema from these
//! fixtures, runs the Python codegen against it, and writes the
//! result to `target/codegen-coverage-client/`. CI imports that
//! generated package — the strict `_rebuild_models()` raises on any
//! dangling type reference.

use std::sync::Arc;

#[derive(Debug, Default)]
struct State;

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

async fn order_coverage(
    _: Arc<State>,
    request: OrderCoverageRequest,
    _headers: reflectapi::Empty,
) -> Result<OrderCoverageResponse, reflectapi::Infallible> {
    let _ = request;
    Ok(OrderCoverageResponse { ok: true })
}

async fn codegen_coverage(
    _: Arc<State>,
    request: coverage::CoverageRequest,
    _headers: reflectapi::Empty,
) -> Result<coverage::CoverageResponse, reflectapi::Infallible> {
    let _ = request;
    Ok(coverage::CoverageResponse { ok: true })
}

fn builder() -> reflectapi::Builder<Arc<State>> {
    reflectapi::Builder::new()
        .name("Codegen coverage")
        .description("Internal test schema for Python codegen smoke tests")
        .route(order_coverage, |b| {
            b.name("coverage.order")
                .description("Coverage fixtures for namespace/tuple/Duration/PhantomData rendering")
        })
        .route(codegen_coverage, |b| {
            b.name("coverage.edges")
                .description("Coverage fixtures for codegen edge cases")
        })
}

#[test]
fn write_python_client() {
    let (schema, _) = builder().build().unwrap();
    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            package_name: "codegen_coverage_client".into(),
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            format: true,
            base_url: None,
        },
    )
    .unwrap();

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("codegen-coverage-client")
        .join("codegen_coverage_client");
    // Wipe between runs so removed files don't linger as stale orphans.
    let _ = std::fs::remove_dir_all(&out_dir);
    for (filename, src) in files {
        let path = out_dir.join(filename);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
}

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

mod coverage {
    use std::collections::HashMap;

    // ---- Python-keyword and builtin field names ----
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
        pub r#type: String,
        pub r#match: String,
    }

    // ---- Pydantic-reserved field names ----
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

    // ---- HashMap with non-string keys ----
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct IntKeyedMap {
        pub by_id: HashMap<u64, String>,
        pub uuid_keyed: HashMap<String, String>,
    }

    // ---- Transparent newtype ----
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

    // ---- Field names shared with module imports ----
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
