use std::sync::Arc;

use futures_util::StreamExt;
use reflectapi::dispatch::{Dispatcher, Reply};
use reflectapi::{DispatchOnly, WithSchema};
use reflectapi_demo::AppState;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn dispatcher() -> Result<Dispatcher<Arc<AppState>>, reflectapi::BuildErrors> {
    Ok(Dispatcher::new(
        reflectapi_demo::api::<DispatchOnly>().build()?,
    ))
}

fn json_headers() -> http::HeaderMap {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        "content-type",
        http::HeaderValue::from_static("application/json"),
    );
    headers.insert("authorization", http::HeaderValue::from_static("password"));
    headers
}

#[test]
fn dispatch_only_builds_the_same_routes_as_with_schema() -> TestResult {
    let (_schema, routers) = reflectapi_demo::api::<WithSchema>().build()?;
    assert_eq!(
        format!("{:?}", Dispatcher::new(routers)),
        format!("{:?}", dispatcher()?)
    );
    Ok(())
}

#[tokio::test]
async fn stream_route_replies_with_a_stream_of_serialized_items() -> TestResult {
    let dispatcher = dispatcher()?;
    let state = Arc::<AppState>::default();
    let headers = json_headers();

    let reply = dispatcher
        .dispatch(
            state.clone(),
            &http::Method::POST,
            "/pets.cdc-events",
            &headers,
            bytes::Bytes::new(),
        )
        .await
        .ok_or("no route")?;
    let Reply::Stream(mut events) = reply else {
        return Err(format!("expected a stream, got {reply:?}").into());
    };

    let created = dispatcher
        .dispatch(
            state,
            &http::Method::POST,
            "/pets.create",
            &headers,
            bytes::Bytes::from_static(
                br#"{"name":"Tweety","kind":{"type":"bird"},"behaviors":[]}"#,
            ),
        )
        .await
        .ok_or("no route")?;
    assert!(
        matches!(created, Reply::Complete { status, .. } if status == http::StatusCode::OK),
        "{created:?}"
    );

    let event = events.next().await.ok_or("stream ended")??;
    let pet: serde_json::Value = serde_json::from_str(&event.data)?;
    assert_eq!(pet["name"], "Tweety");
    assert_eq!(event.id, None);
    assert_eq!(
        reflectapi::dispatch::sse_event(&event),
        format!("data: {}\n\n", event.data)
    );
    Ok(())
}

mod events {
    use futures_util::Stream;

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Tick {
        pub seq: u32,
    }

    /// Ticks 1 and 2 with their `seq` as the event id, then one without an id.
    pub async fn ticks(
        _: (),
        _: reflectapi::Empty,
        _: reflectapi::Empty,
    ) -> Result<impl Stream<Item = reflectapi::Event<Tick>>, reflectapi::Infallible> {
        Ok(futures_util::stream::iter(
            [1, 2]
                .map(|seq| reflectapi::Event::new(seq, Tick { seq }))
                .into_iter()
                .chain([reflectapi::Event {
                    id: None,
                    data: Tick { seq: 3 },
                }]),
        ))
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().stream_route(ticks, |b| b.name("ticks"))
    }
}

#[tokio::test]
async fn stream_route_sends_the_ids_of_events() -> TestResult {
    let dispatcher = Dispatcher::new(events::api::<DispatchOnly>().build()?);
    let reply = dispatcher
        .dispatch(
            (),
            &http::Method::POST,
            "/ticks",
            &json_headers(),
            bytes::Bytes::new(),
        )
        .await
        .ok_or("no route")?;
    let Reply::Stream(events) = reply else {
        return Err(format!("expected a stream, got {reply:?}").into());
    };

    let framed = events
        .map(|event| event.map(|event| reflectapi::dispatch::sse_event(&event)))
        .collect::<Vec<_>>()
        .await;
    assert_eq!(
        framed,
        [
            Ok("id: 1\ndata: {\"seq\":1}\n\n".to_string()),
            Ok("id: 2\ndata: {\"seq\":2}\n\n".to_string()),
            Ok("data: {\"seq\":3}\n\n".to_string()),
        ]
    );
    Ok(())
}

#[test]
fn the_schema_shows_the_item_of_an_event() -> TestResult {
    let (schema, _) = events::api::<WithSchema>().build()?;
    let function = schema.functions().next().ok_or("no function")?;
    let reflectapi::OutputType::Stream { item_type } = &function.output_type else {
        return Err(format!("expected a stream, got {:?}", function.output_type).into());
    };
    assert!(item_type.name.ends_with("Tick"), "{}", item_type.name);
    Ok(())
}

/// The axum adapter sends events as `sse_event` frames them.
#[tokio::test]
async fn event_ids_match_axum() -> TestResult {
    use tower::Service as _;

    let (_schema, routers) = events::api::<WithSchema>().build()?;
    let mut app = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    let response = app
        .call(
            http::Request::builder()
                .method(http::Method::POST)
                .uri("/ticks")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    let mut frames = response.into_body().into_data_stream();
    let mut sent = String::new();
    while let Some(frame) = frames.next().await {
        sent.push_str(std::str::from_utf8(&frame?)?);
    }
    assert_eq!(
        sent,
        "id: 1\ndata: {\"seq\":1}\n\nid: 2\ndata: {\"seq\":2}\n\ndata: {\"seq\":3}\n\n"
    );
    Ok(())
}

mod quiet {
    use futures_util::{Stream, StreamExt};

    /// One tick, then nothing, without ending.
    pub async fn quiet(
        _: (),
        _: reflectapi::Empty,
        _: reflectapi::Empty,
    ) -> Result<impl Stream<Item = super::events::Tick>, reflectapi::Infallible> {
        Ok(futures_util::stream::iter([super::events::Tick { seq: 1 }])
            .chain(futures_util::stream::pending()))
    }

    pub fn api() -> reflectapi::Builder<()> {
        reflectapi::Builder::new().stream_route(quiet, |b| b.name("quiet"))
    }

    /// `/quiet`, with its own keep-alive interval of 5s.
    pub fn api_keeping_alive() -> reflectapi::Builder<()> {
        reflectapi::Builder::new().stream_route(quiet, |b| {
            b.name("quiet")
                .sse_keep_alive(std::time::Duration::from_secs(5))
        })
    }

    /// `/quiet`, with keep-alive turned off.
    pub fn api_never_keeping_alive() -> reflectapi::Builder<()> {
        reflectapi::Builder::new().stream_route(quiet, |b| {
            b.name("quiet").sse_keep_alive(std::time::Duration::ZERO)
        })
    }
}

/// Posts to `path` on an axum app that keeps idle streams alive every 15s.
async fn kept_alive_body(
    routers: Vec<reflectapi::Router<()>>,
    path: &str,
) -> Result<axum::body::BodyDataStream, Box<dyn std::error::Error>> {
    let settings = reflectapi::axum::RouterSettings::default()
        .sse_keep_alive(std::time::Duration::from_secs(15), tokio::time::sleep);
    served_body(routers, settings, path).await
}

/// Posts to `path` on an axum app served with `settings`.
async fn served_body(
    routers: Vec<reflectapi::Router<()>>,
    settings: reflectapi::axum::RouterSettings,
    path: &str,
) -> Result<axum::body::BodyDataStream, Box<dyn std::error::Error>> {
    use tower::Service as _;

    let mut app = reflectapi::axum::into_router((), routers, settings, |_, r| r);
    let response = app
        .call(
            http::Request::builder()
                .method(http::Method::POST)
                .uri(path)
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.headers()[http::header::CONTENT_TYPE],
        "text/event-stream"
    );
    Ok(response.into_body().into_data_stream())
}

async fn next_frame(
    frames: &mut axum::body::BodyDataStream,
) -> Result<String, Box<dyn std::error::Error>> {
    let frame = frames.next().await.ok_or("the stream ended")??;
    Ok(std::str::from_utf8(&frame)?.to_owned())
}

/// An idle stream gets a comment every interval, which SSE clients ignore.
#[tokio::test(start_paused = true)]
async fn an_idle_axum_stream_is_kept_alive() -> TestResult {
    let (_schema, routers) = quiet::api().build()?;
    let mut frames = kept_alive_body(routers, "/quiet").await?;

    let start = tokio::time::Instant::now();
    assert_eq!(next_frame(&mut frames).await?, "data: {\"seq\":1}\n\n");
    assert_eq!(next_frame(&mut frames).await?, ":\n\n");
    assert_eq!(start.elapsed(), std::time::Duration::from_secs(15));
    assert_eq!(next_frame(&mut frames).await?, ":\n\n");
    assert_eq!(start.elapsed(), std::time::Duration::from_secs(30));
    Ok(())
}

/// A stream that is never idle for the interval carries no comments.
#[tokio::test(start_paused = true)]
async fn a_busy_axum_stream_carries_no_comments() -> TestResult {
    let (_schema, routers) = events::api::<WithSchema>().build()?;
    let mut frames = kept_alive_body(routers, "/ticks").await?;
    let mut sent = String::new();
    while let Some(frame) = frames.next().await {
        sent.push_str(std::str::from_utf8(&frame?)?);
    }
    assert_eq!(
        sent,
        "id: 1\ndata: {\"seq\":1}\n\nid: 2\ndata: {\"seq\":2}\n\ndata: {\"seq\":3}\n\n"
    );
    Ok(())
}

/// A route's own interval needs only a timer from the server.
#[tokio::test(start_paused = true)]
async fn a_route_sets_its_own_keep_alive_interval() -> TestResult {
    let (_schema, routers) = quiet::api_keeping_alive().build()?;
    let settings = reflectapi::axum::RouterSettings::default().sse_timer(tokio::time::sleep);
    let mut frames = served_body(routers, settings, "/quiet").await?;

    let start = tokio::time::Instant::now();
    assert_eq!(next_frame(&mut frames).await?, "data: {\"seq\":1}\n\n");
    assert_eq!(next_frame(&mut frames).await?, ":\n\n");
    assert_eq!(start.elapsed(), std::time::Duration::from_secs(5));
    Ok(())
}

/// A route's interval wins over the server's, so it can be shorter.
#[tokio::test(start_paused = true)]
async fn a_route_interval_overrides_the_server_default() -> TestResult {
    let (_schema, routers) = quiet::api_keeping_alive().build()?;
    let mut frames = kept_alive_body(routers, "/quiet").await?;

    let start = tokio::time::Instant::now();
    assert_eq!(next_frame(&mut frames).await?, "data: {\"seq\":1}\n\n");
    assert_eq!(next_frame(&mut frames).await?, ":\n\n");
    assert_eq!(start.elapsed(), std::time::Duration::from_secs(5));
    Ok(())
}

/// `Duration::ZERO` on a route turns off the server's keep-alive for it.
#[tokio::test(start_paused = true)]
async fn a_zero_route_interval_turns_keep_alive_off() -> TestResult {
    let (_schema, routers) = quiet::api_never_keeping_alive().build()?;
    let mut frames = kept_alive_body(routers, "/quiet").await?;

    assert_eq!(next_frame(&mut frames).await?, "data: {\"seq\":1}\n\n");
    let idle = tokio::time::timeout(std::time::Duration::from_secs(60), frames.next()).await;
    assert!(idle.is_err(), "expected nothing, got {idle:?}");
    Ok(())
}

#[test]
#[should_panic(expected = "RouterSettings have no timer")]
fn a_route_interval_without_a_timer_panics_when_served() {
    let (_schema, routers) = quiet::api_keeping_alive().build().unwrap();
    let _ = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
}

#[test]
fn a_zero_route_interval_needs_no_timer() -> TestResult {
    let (_schema, routers) = quiet::api_never_keeping_alive().build()?;
    let _ = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    Ok(())
}

#[test]
fn the_dispatcher_reports_a_route_interval() -> TestResult {
    let dispatcher = Dispatcher::new(quiet::api_keeping_alive().build()?.1);
    assert_eq!(
        dispatcher.sse_keep_alive("/quiet"),
        Some(std::time::Duration::from_secs(5))
    );

    let dispatcher = Dispatcher::new(quiet::api().build()?.1);
    assert_eq!(dispatcher.sse_keep_alive("/quiet"), None);
    assert_eq!(dispatcher.sse_keep_alive("/nowhere"), None);
    Ok(())
}

#[test]
fn keep_alive_on_a_route_that_is_not_a_stream_is_a_build_error() -> TestResult {
    let result = reflectapi::Builder::<()>::new()
        .route(invalid_headers::handler, |b| {
            b.name("plain")
                .sse_keep_alive(std::time::Duration::from_secs(5))
        })
        .build();
    let Err(errors) = result else {
        return Err("expected a build error".into());
    };
    let errors = errors.to_string();
    assert!(
        errors.contains("route `plain`: `sse_keep_alive` applies only to stream routes"),
        "{errors}"
    );
    Ok(())
}

#[tokio::test]
async fn unknown_paths_and_unaccepted_methods() -> TestResult {
    let dispatcher = dispatcher()?;
    let state = Arc::<AppState>::default();
    let headers = json_headers();

    let unknown = dispatcher
        .dispatch(
            state.clone(),
            &http::Method::POST,
            "/pets.adopt",
            &headers,
            bytes::Bytes::new(),
        )
        .await;
    assert!(unknown.is_none());

    // pets.create is not readonly: POST only.
    let Some(Reply::Complete {
        status, headers, ..
    }) = dispatcher
        .dispatch(
            state,
            &http::Method::GET,
            "/pets.create",
            &json_headers(),
            bytes::Bytes::new(),
        )
        .await
    else {
        return Err("expected a complete reply".into());
    };
    assert_eq!(status, http::StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(headers.get(http::header::ALLOW).ok_or("no allow")?, "POST");
    Ok(())
}

mod invalid_headers {
    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Headers {
        #[serde(rename = "not a header")]
        pub value: String,
    }

    pub async fn handler(
        _: (),
        _: reflectapi::Empty,
        headers: Headers,
    ) -> Result<String, reflectapi::Infallible> {
        Ok(headers.value)
    }
}

#[test]
fn an_invalid_request_header_name_is_a_build_error_in_both_modes() -> TestResult {
    let with_schema = reflectapi::Builder::<(), WithSchema>::default()
        .route(invalid_headers::handler, |b| b.name("invalid"))
        .build();
    let dispatch_only = reflectapi::Builder::<(), DispatchOnly>::default()
        .route(invalid_headers::handler, |b| b.name("invalid"))
        .build();

    for result in [with_schema.map(|_| ()), dispatch_only.map(|_| ())] {
        let Err(errors) = result else {
            return Err("expected a build error".into());
        };
        let errors = errors.to_string();
        assert!(
            errors.contains("`not a header` is not a valid header name"),
            "{errors}"
        );
    }
    Ok(())
}

#[test]
fn dispatch_only_leaves_schema_validation_to_the_schema_build() {
    fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default()
            .name("validated")
            .validate(|_| {
                vec![reflectapi::ValidationError::new(
                    reflectapi::ValidationPointer::Type("any".into()),
                    "always fails".into(),
                )]
            })
            .rename_types("no::such::prefix::", "")
    }

    assert!(api::<WithSchema>().build().is_err());
    assert!(api::<DispatchOnly>().build().is_ok());
}

/// The axum adapter and the dispatcher answer an unknown path and an
/// unaccepted method the same way.
#[tokio::test]
async fn unknown_paths_and_methods_match_axum() -> TestResult {
    use tower::Service as _;

    let (_schema, routers) = reflectapi_demo::api::<WithSchema>().build()?;
    let mut app = reflectapi::axum::into_router(
        Arc::<AppState>::default(),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    let dispatcher = dispatcher()?;

    for (method, path) in [
        (http::Method::GET, "/pets.create"), // POST only
        (http::Method::POST, "/pets.adopt"), // no such route
    ] {
        let mut request = http::Request::builder().method(method.clone()).uri(path);
        for (name, value) in &json_headers() {
            request = request.header(name, value);
        }
        let axum_response = app.call(request.body(axum::body::Body::empty())?).await?;
        let axum_status = axum_response.status();
        let mut axum_headers = axum_response.headers().clone();
        axum_headers.remove(http::header::CONTENT_LENGTH);
        let axum_body = axum::body::to_bytes(axum_response.into_body(), 10_000).await?;

        let dispatched = dispatcher
            .dispatch(
                Arc::default(),
                &method,
                path,
                &json_headers(),
                bytes::Bytes::new(),
            )
            .await;
        let (status, headers, body) = match dispatched {
            None => (
                http::StatusCode::NOT_FOUND,
                axum_headers.clone(),
                axum_body.clone(),
            ),
            Some(Reply::Complete {
                status,
                headers,
                body,
            }) => (status, headers, body),
            Some(Reply::Stream(_)) => return Err(format!("{method} {path}: stream").into()),
        };
        assert_eq!(status, axum_status, "{method} {path}");
        assert_eq!(headers, axum_headers, "{method} {path}");
        assert_eq!(body, axum_body, "{method} {path}");
    }
    Ok(())
}

/// Readonly routes accept `GET` and `POST` only. Unlike axum, which runs the
/// `GET` handler for `HEAD` and drops the body, the dispatcher answers `405`.
#[tokio::test]
async fn readonly_routes_reject_other_methods_including_head() -> TestResult {
    let dispatcher = dispatcher()?;
    for method in [http::Method::PUT, http::Method::HEAD] {
        let dispatched = dispatcher
            .dispatch(
                Arc::default(),
                &method,
                "/pets.list",
                &json_headers(),
                bytes::Bytes::new(),
            )
            .await;
        let Some(Reply::Complete {
            status,
            headers,
            body,
        }) = dispatched
        else {
            return Err(format!("{method}: expected a complete reply").into());
        };
        assert_eq!(status, http::StatusCode::METHOD_NOT_ALLOWED, "{method}");
        assert_eq!(
            headers.get(http::header::ALLOW).unwrap(),
            "GET,POST",
            "{method}"
        );
        assert!(body.is_empty(), "{method}");
    }
    Ok(())
}

mod waiting {
    use futures_util::Stream;

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Ask {
        pub refuse: bool,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Refused {
        pub reason: String,
    }

    impl reflectapi::StatusCode for Refused {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::CONFLICT
        }
    }

    /// Waits before it decides, then refuses, or streams one tick.
    pub async fn ticks(
        _: (),
        ask: Ask,
        _: reflectapi::Empty,
    ) -> Result<impl Stream<Item = super::events::Tick>, Refused> {
        tokio::task::yield_now().await;
        if ask.refuse {
            return Err(Refused {
                reason: "asked to".into(),
            });
        }
        Ok(futures_util::stream::iter([super::events::Tick { seq: 1 }]))
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default().stream_route(ticks, |b| b.name("ticks"))
    }
}

#[tokio::test]
async fn a_stream_route_waits_before_it_streams_or_refuses() -> TestResult {
    let dispatcher = Dispatcher::new(waiting::api::<DispatchOnly>().build()?);
    let headers = json_headers();
    let ask = |refuse: &'static [u8]| {
        dispatcher.dispatch(
            (),
            &http::Method::POST,
            "/ticks",
            &headers,
            bytes::Bytes::from_static(refuse),
        )
    };

    let streamed = ask(br#"{"refuse":false}"#).await.ok_or("no route")?;
    let Reply::Stream(events) = streamed else {
        return Err(format!("expected a stream, got {streamed:?}").into());
    };
    let data = events
        .map(|event| event.map(|event| event.data))
        .collect::<Vec<_>>()
        .await;
    assert_eq!(data, [Ok(r#"{"seq":1}"#.to_string())]);

    let refused = ask(br#"{"refuse":true}"#).await.ok_or("no route")?;
    let Reply::Complete { status, body, .. } = refused else {
        return Err(format!("expected a refusal, got {refused:?}").into());
    };
    assert_eq!(status, http::StatusCode::CONFLICT);
    assert_eq!(body, r#"{"reason":"asked to"}"#);
    Ok(())
}

#[tokio::test]
async fn a_stream_route_waits_before_it_refuses_through_axum() -> TestResult {
    use tower::Service as _;

    let (_schema, routers) = waiting::api::<WithSchema>().build()?;
    let mut app = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    let response = app
        .call(
            http::Request::builder()
                .method(http::Method::POST)
                .uri("/ticks")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#"{"refuse":true}"#))?,
        )
        .await?;
    assert_eq!(response.status(), http::StatusCode::CONFLICT);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(body, r#"{"reason":"asked to"}"#);
    Ok(())
}

mod flattened_headers {
    /// A route's headers, its own flattened in: the shape a framework built
    /// on reflectapi gives its routes.
    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct RoutingHeaders<T = reflectapi::Empty> {
        #[serde(rename = "x-cell")]
        pub cell: Option<String>,
        #[serde(flatten)]
        pub headers: T,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct AuthorizationHeaders {
        pub authorization: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Seen {
        pub cell: Option<String>,
        pub authorization: Option<String>,
    }

    pub async fn with_authorization(
        _: (),
        _: reflectapi::Empty,
        headers: RoutingHeaders<AuthorizationHeaders>,
    ) -> Seen {
        Seen {
            cell: headers.cell,
            authorization: Some(headers.headers.authorization),
        }
    }

    pub async fn without(_: (), _: reflectapi::Empty, headers: RoutingHeaders) -> Seen {
        Seen {
            cell: headers.cell,
            authorization: None,
        }
    }

    pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<(), M> {
        reflectapi::Builder::default()
            .route(with_authorization, |b| b.name("with-authorization"))
            .route(without, |b| b.name("without"))
    }

    pub fn headers() -> http::HeaderMap {
        let mut headers = super::json_headers();
        headers.insert("x-cell", http::HeaderValue::from_static("acme"));
        headers.insert("authorization", http::HeaderValue::from_static("secret"));
        headers
    }
}

#[tokio::test]
async fn a_flattened_headers_field_passes_its_own_headers() -> TestResult {
    let dispatcher = Dispatcher::new(flattened_headers::api::<DispatchOnly>().build()?);
    let headers = flattened_headers::headers();
    let seen = |path: &'static str| {
        dispatcher.dispatch((), &http::Method::POST, path, &headers, bytes::Bytes::new())
    };

    let reply = seen("/with-authorization").await.ok_or("no route")?;
    let Reply::Complete { status, body, .. } = reply else {
        return Err(format!("expected a response, got {reply:?}").into());
    };
    assert_eq!(status, http::StatusCode::OK, "{body:?}");
    assert_eq!(body, r#"{"cell":"acme","authorization":"secret"}"#);

    let reply = seen("/without").await.ok_or("no route")?;
    let Reply::Complete { status, body, .. } = reply else {
        return Err(format!("expected a response, got {reply:?}").into());
    };
    assert_eq!(status, http::StatusCode::OK, "{body:?}");
    assert_eq!(body, r#"{"cell":"acme","authorization":null}"#);
    Ok(())
}

#[tokio::test]
async fn a_flattened_headers_field_passes_its_own_headers_through_axum() -> TestResult {
    use tower::Service as _;

    let (_schema, routers) = flattened_headers::api::<WithSchema>().build()?;
    let mut app = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_, r| r,
    );
    let mut request = http::Request::builder()
        .method(http::Method::POST)
        .uri("/with-authorization")
        .body(axum::body::Body::empty())?;
    *request.headers_mut() = flattened_headers::headers();
    let response = app.call(request).await?;
    assert_eq!(response.status(), http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(body, r#"{"cell":"acme","authorization":"secret"}"#);
    Ok(())
}

#[test]
fn openapi_describes_a_flattened_headers_field_as_header_parameters() -> TestResult {
    let (schema, _) = flattened_headers::api::<WithSchema>().build()?;
    let spec = serde_json::to_value(reflectapi::codegen::openapi::generate_spec(
        &schema,
        &reflectapi::codegen::openapi::Config::default(),
    ))?;
    let parameters = |path: &str| {
        spec["paths"][path]["post"]["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|parameter| {
                assert_eq!(parameter["in"], "header", "{parameter}");
                (
                    parameter["name"].as_str().unwrap_or_default().to_owned(),
                    parameter["required"].as_bool().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        parameters("/with-authorization"),
        [
            ("authorization".to_owned(), true),
            ("x-cell".to_owned(), false)
        ]
    );
    assert_eq!(parameters("/without"), [("x-cell".to_owned(), false)]);
    Ok(())
}

mod flattened_enum_headers {
    #[derive(serde::Deserialize, reflectapi::Input)]
    #[serde(tag = "kind")]
    pub enum Auth {
        Bearer { token: String },
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Headers {
        #[serde(flatten)]
        pub auth: Auth,
    }

    pub async fn handler(
        _: (),
        _: reflectapi::Empty,
        headers: Headers,
    ) -> Result<String, reflectapi::Infallible> {
        let Auth::Bearer { token } = headers.auth;
        Ok(token)
    }
}

#[test]
fn a_flattened_headers_field_that_is_not_a_struct_is_a_build_error_in_both_modes() -> TestResult {
    let with_schema = reflectapi::Builder::<(), WithSchema>::default()
        .route(flattened_enum_headers::handler, |b| b.name("enum"))
        .build();
    let dispatch_only = reflectapi::Builder::<(), DispatchOnly>::default()
        .route(flattened_enum_headers::handler, |b| b.name("enum"))
        .build();

    for result in [with_schema.map(|_| ()), dispatch_only.map(|_| ())] {
        let Err(errors) = result else {
            return Err("expected a build error".into());
        };
        let errors = errors.to_string();
        assert!(
            errors.contains(
                "flattened field `auth` holds a type other than a struct with named fields"
            ),
            "{errors}"
        );
    }
    Ok(())
}

#[test]
fn clients_send_a_flattened_headers_field() -> TestResult {
    let (schema, _) = flattened_headers::api::<WithSchema>().build()?;

    let rust = reflectapi::codegen::rust::generate(
        schema.clone(),
        &reflectapi::codegen::rust::Config::default(),
    )?;
    assert!(rust.contains("authorization"), "{rust}");

    let typescript = reflectapi::codegen::typescript::generate(
        schema.clone(),
        &reflectapi::codegen::typescript::Config::default(),
    )?;
    assert!(
        typescript
            .values()
            .any(|file| file.contains("authorization")),
        "{typescript:?}"
    );

    let python = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )?;
    assert!(
        python.values().any(|file| file.contains("authorization")),
        "{python:?}"
    );
    Ok(())
}
