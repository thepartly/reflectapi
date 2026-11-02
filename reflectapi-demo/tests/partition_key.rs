use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use reflectapi::dispatch::{Dispatcher, Reply};
use reflectapi::{DispatchOnly, WithSchema};

type TestResult = Result<(), Box<dyn std::error::Error>>;

mod orders {
    use std::sync::{atomic::Ordering, Arc};

    use super::Calls;

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct TenantHeaders {
        #[reflectapi(partition_key = 1)]
        #[serde(rename = "x-tenant")]
        pub tenant: String,
    }

    /// An input and an output, so the schema keeps one type for both.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct PayRequest {
        #[reflectapi(partition_key = 2)]
        pub order: String,
        pub amount: u64,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct ByAccount {
        #[reflectapi(partition_key)]
        pub account: u64,
    }

    pub async fn pay(
        calls: Arc<Calls>,
        input: PayRequest,
        _: TenantHeaders,
    ) -> Result<PayRequest, reflectapi::Infallible> {
        calls.0.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }

    pub async fn balance(
        calls: Arc<Calls>,
        input: ByAccount,
        _: reflectapi::Empty,
    ) -> Result<u64, reflectapi::Infallible> {
        calls.0.fetch_add(1, Ordering::SeqCst);
        Ok(input.account)
    }

    pub async fn health(
        calls: Arc<Calls>,
        _: reflectapi::Empty,
        _: reflectapi::Empty,
    ) -> Result<(), reflectapi::Infallible> {
        calls.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<Arc<Calls>, M> {
        reflectapi::Builder::default()
            .name("orders")
            .route(pay, |b| b.name("pay"))
            .route(balance, |b| b.name("balance").readonly(true))
            .route(health, |b| b.name("health"))
    }
}

/// How many times a handler ran.
#[derive(Default)]
pub struct Calls(AtomicUsize);

fn dispatcher() -> Result<Dispatcher<Arc<Calls>>, reflectapi::BuildErrors> {
    Ok(Dispatcher::new(orders::api::<DispatchOnly>().build()?))
}

fn headers(tenant: &str) -> http::HeaderMap {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        "content-type",
        http::HeaderValue::from_static("application/json"),
    );
    headers.insert("x-tenant", http::HeaderValue::from_str(tenant).unwrap());
    headers
}

fn pay_body(order: &str) -> bytes::Bytes {
    format!(r#"{{"order":"{order}","amount":10}}"#).into()
}

fn status(reply: Option<Reply>) -> Result<http::StatusCode, String> {
    match reply {
        Some(Reply::Complete { status, .. }) => Ok(status),
        other => Err(format!("expected a complete reply, got {other:?}")),
    }
}

#[test]
fn the_schema_gives_each_marked_field_its_position() -> TestResult {
    let (schema, _) = orders::api::<WithSchema>().build()?;
    let position = |type_suffix: &str, field: &str| -> Result<Option<u8>, String> {
        let ty = schema
            .input_types()
            .types()
            .find(|ty| ty.name().ends_with(type_suffix))
            .ok_or(format!("no type {type_suffix}"))?;
        let reflectapi::Type::Struct(strukt) = ty else {
            return Err(format!("{type_suffix} is not a struct"));
        };
        let field = strukt
            .fields()
            .find(|f| f.serde_name() == field)
            .ok_or(format!("no field {field}"))?;
        Ok(field.partition_key())
    };
    assert_eq!(position("TenantHeaders", "x-tenant")?, Some(1));
    assert_eq!(position("PayRequest", "order")?, Some(2));
    assert_eq!(position("PayRequest", "amount")?, None);
    // A bare `partition_key` is position 1.
    assert_eq!(position("ByAccount", "account")?, Some(1));

    // Recorded in both directions, so a type that is an input and an output
    // is still one type, not split into `input::` and `output::` ones.
    let name = "partition_key::orders::PayRequest";
    assert_eq!(
        schema.input_types().get_type(name),
        schema.output_types().get_type(name)
    );
    assert!(schema.input_types().get_type(name).is_some());

    let json = serde_json::to_value(&schema)?;
    let fields = json.to_string();
    assert!(fields.contains(r#""partition_key":2"#), "{fields}");
    Ok(())
}

#[test]
fn partition_key_gives_the_parts_in_position_order() -> TestResult {
    let dispatcher = dispatcher()?;
    let key = dispatcher.partition_key(
        &http::Method::POST,
        "/pay",
        &headers("acme"),
        pay_body("o-42"),
    );
    assert_eq!(key.ok(), Some(Some(vec!["acme".into(), "o-42".into()])));

    let key = dispatcher.partition_key(
        &http::Method::GET,
        "/balance",
        &headers("acme"),
        r#"{"account":7}"#.into(),
    );
    assert_eq!(key.ok(), Some(Some(vec!["7".into()])));
    Ok(())
}

#[test]
fn partition_key_is_none_where_dispatch_calls_no_keyed_handler() -> TestResult {
    let dispatcher = dispatcher()?;
    for (method, path) in [
        (http::Method::POST, "/health"), // no partition key
        (http::Method::POST, "/refund"), // no such route
        (http::Method::GET, "/pay"),     // POST only
    ] {
        let key = dispatcher.partition_key(&method, path, &headers("acme"), pay_body("o-42"));
        assert_eq!(key.ok(), Some(None), "{method} {path}");
    }
    Ok(())
}

#[test]
fn partition_key_answers_as_the_handler_for_input_that_does_not_parse() -> TestResult {
    let dispatcher = dispatcher()?;
    let Err(Reply::Complete { status, body, .. }) = dispatcher.partition_key(
        &http::Method::POST,
        "/pay",
        &headers("acme"),
        r#"{"amount":10}"#.into(),
    ) else {
        return Err("expected an error reply".into());
    };
    assert_eq!(status, http::StatusCode::BAD_REQUEST);
    assert!(
        String::from_utf8_lossy(&body).contains("missing field `order`"),
        "{body:?}"
    );

    // The headers are parsed as well.
    let mut without_tenant = headers("acme");
    without_tenant.remove("x-tenant");
    let Err(Reply::Complete { status, .. }) = dispatcher.partition_key(
        &http::Method::POST,
        "/pay",
        &without_tenant,
        pay_body("o-42"),
    ) else {
        return Err("expected an error reply".into());
    };
    assert_eq!(status, http::StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn dispatch_in_calls_the_handler_for_the_partitions_requests() -> TestResult {
    let dispatcher = dispatcher()?;
    let calls = Arc::<Calls>::default();
    let reply = dispatcher
        .dispatch_in(
            &["acme", "o-42"],
            calls.clone(),
            &http::Method::POST,
            "/pay",
            &headers("acme"),
            pay_body("o-42"),
        )
        .await;
    assert_eq!(status(reply)?, http::StatusCode::OK);
    assert_eq!(calls.0.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn dispatch_in_refuses_another_partitions_requests_without_calling_the_handler() -> TestResult
{
    let dispatcher = dispatcher()?;
    let calls = Arc::<Calls>::default();
    for (partition, tenant, order) in [
        (&["acme", "o-42"][..], "globex", "o-42"), // another tenant
        (&["acme", "o-42"][..], "acme", "o-43"),   // another order
        (&["acme"][..], "acme", "o-42"),           // fewer parts
        (&["acme", "o-42", "x"][..], "acme", "o-42"), // more parts
    ] {
        let reply = dispatcher
            .dispatch_in(
                partition,
                calls.clone(),
                &http::Method::POST,
                "/pay",
                &headers(tenant),
                pay_body(order),
            )
            .await;
        assert_eq!(
            status(reply)?,
            http::StatusCode::BAD_REQUEST,
            "{partition:?} {tenant} {order}"
        );
    }
    assert_eq!(calls.0.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn dispatch_in_runs_a_route_without_a_key_in_any_partition() -> TestResult {
    let dispatcher = dispatcher()?;
    let calls = Arc::<Calls>::default();
    let reply = dispatcher
        .dispatch_in(
            &["anything"],
            calls.clone(),
            &http::Method::POST,
            "/health",
            &headers("acme"),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(status(reply)?, http::StatusCode::OK);
    assert_eq!(calls.0.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn dispatch_does_not_check_the_partition() -> TestResult {
    let dispatcher = dispatcher()?;
    let calls = Arc::<Calls>::default();
    for (tenant, order) in [("acme", "o-42"), ("globex", "o-43")] {
        let reply = dispatcher
            .dispatch(
                calls.clone(),
                &http::Method::POST,
                "/pay",
                &headers(tenant),
                pay_body(order),
            )
            .await;
        assert_eq!(status(reply)?, http::StatusCode::OK);
    }
    assert_eq!(calls.0.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn dispatch_in_answers_unknown_paths_and_methods_as_dispatch() -> TestResult {
    let dispatcher = dispatcher()?;
    let unknown = dispatcher
        .dispatch_in(
            &["acme", "o-42"],
            Arc::default(),
            &http::Method::POST,
            "/refund",
            &headers("acme"),
            pay_body("o-42"),
        )
        .await;
    assert!(unknown.is_none());

    let reply = dispatcher
        .dispatch_in(
            &["acme", "o-42"],
            Arc::default(),
            &http::Method::GET,
            "/pay",
            &headers("acme"),
            pay_body("o-42"),
        )
        .await;
    assert_eq!(status(reply)?, http::StatusCode::METHOD_NOT_ALLOWED);
    Ok(())
}

mod streams {
    use futures_util::Stream;

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Watch {
        #[reflectapi(partition_key)]
        pub order: String,
    }

    pub fn watch(
        _: (),
        input: Watch,
        _: reflectapi::Empty,
    ) -> Result<impl Stream<Item = String>, reflectapi::Infallible> {
        Ok(futures_util::stream::iter([input.order]))
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().stream_route(watch, |b| b.name("watch"))
    }
}

#[tokio::test]
async fn dispatch_in_checks_stream_routes() -> TestResult {
    let dispatcher = Dispatcher::new(streams::api::<DispatchOnly>().build()?);
    let body = || bytes::Bytes::from_static(br#"{"order":"o-42"}"#);
    let key = dispatcher.partition_key(&http::Method::POST, "/watch", &headers("acme"), body());
    assert_eq!(key.ok(), Some(Some(vec!["o-42".into()])));

    let reply = dispatcher
        .dispatch_in(
            &["o-42"],
            (),
            &http::Method::POST,
            "/watch",
            &headers("acme"),
            body(),
        )
        .await;
    assert!(matches!(reply, Some(Reply::Stream(_))), "{reply:?}");

    let reply = dispatcher
        .dispatch_in(
            &["o-43"],
            (),
            &http::Method::POST,
            "/watch",
            &headers("acme"),
            body(),
        )
        .await;
    assert_eq!(status(reply)?, http::StatusCode::BAD_REQUEST);
    Ok(())
}

mod part_types {
    /// A newtype that implements `PartitionKeyPart` itself.
    #[derive(serde::Deserialize, reflectapi::Input)]
    #[serde(transparent)]
    pub struct OrderId(pub String);

    impl reflectapi::PartitionKeyPart for OrderId {
        fn partition_key_part(&self) -> String {
            self.0.clone()
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Generic<T> {
        #[reflectapi(partition_key = 1)]
        pub id: T,
        #[reflectapi(partition_key = 2)]
        pub order: OrderId,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Tuple(#[reflectapi(partition_key)] pub uuid::Uuid);

    pub async fn generic(
        _: (),
        _: Generic<i32>,
        _: reflectapi::Empty,
    ) -> Result<(), reflectapi::Infallible> {
        Ok(())
    }

    pub async fn tuple(
        _: (),
        _: Tuple,
        _: reflectapi::Empty,
    ) -> Result<(), reflectapi::Infallible> {
        Ok(())
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default()
            .route(generic, |b| b.name("generic"))
            .route(tuple, |b| b.name("tuple"))
    }
}

#[test]
fn parts_are_the_values_of_integers_uuids_generics_and_custom_types() -> TestResult {
    let dispatcher = Dispatcher::new(part_types::api::<DispatchOnly>().build()?);
    let key = dispatcher.partition_key(
        &http::Method::POST,
        "/generic",
        &headers("acme"),
        r#"{"id":-3,"order":"o-42"}"#.into(),
    );
    assert_eq!(key.ok(), Some(Some(vec!["-3".into(), "o-42".into()])));

    let key = dispatcher.partition_key(
        &http::Method::POST,
        "/tuple",
        &headers("acme"),
        r#""67e55044-10b1-426f-9247-bb680e5fe0c8""#.into(),
    );
    assert_eq!(
        key.ok(),
        Some(Some(vec!["67e55044-10b1-426f-9247-bb680e5fe0c8".into()]))
    );
    Ok(())
}

mod invalid_routes {
    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Second {
        #[reflectapi(partition_key = 2)]
        pub order: String,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct First {
        #[reflectapi(partition_key)]
        pub tenant: String,
    }

    pub async fn gap(_: (), _: Second, _: reflectapi::Empty) -> Result<(), reflectapi::Infallible> {
        Ok(())
    }

    pub async fn repeat(_: (), _: First, _: First) -> Result<(), reflectapi::Infallible> {
        Ok(())
    }

    pub fn gap_api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().route(gap, |b| b.name("gap"))
    }

    pub fn repeat_api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().route(repeat, |b| b.name("repeat"))
    }
}

#[test]
fn positions_with_a_gap_or_a_repeat_are_a_build_error_in_both_modes() -> TestResult {
    for (errors, expected) in [
        (
            invalid_routes::gap_api::<WithSchema>().build().err(),
            "route `gap`: the partition key positions of its input and headers types, [2],",
        ),
        (
            invalid_routes::gap_api::<DispatchOnly>().build().err(),
            "route `gap`: the partition key positions of its input and headers types, [2],",
        ),
        (
            invalid_routes::repeat_api::<WithSchema>().build().err(),
            "route `repeat`: the partition key positions of its input and headers types, [1, 1],",
        ),
        (
            invalid_routes::repeat_api::<DispatchOnly>().build().err(),
            "route `repeat`: the partition key positions of its input and headers types, [1, 1],",
        ),
    ] {
        let errors = errors.ok_or("expected a build error")?.to_string();
        assert!(errors.contains(expected), "{errors}");
    }
    Ok(())
}
