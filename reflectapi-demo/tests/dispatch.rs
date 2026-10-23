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
    pub fn ticks(
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
    let mut app = reflectapi::axum::into_router((), routers, |_, r| r);
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
    let mut app = reflectapi::axum::into_router(Arc::<AppState>::default(), routers, |_, r| r);
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
