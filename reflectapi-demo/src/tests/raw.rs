//! `reflectapi::Raw` through a route, in JSON and MessagePack.

#[derive(
    Debug, PartialEq, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
)]
struct Event {
    kind: String,
    amount: u64,
}

async fn forward(
    _: (),
    body: reflectapi::Raw<Event>,
    _: reflectapi::Empty,
) -> reflectapi::Raw<Event> {
    // A proxy's access check: read one field, forward the rest unchanged.
    assert_eq!(body.peek::<serde_json::Value>().unwrap()["kind"], "refund");
    body
}

async fn call(content_type: &str, body: Vec<u8>) -> (http::StatusCode, Vec<u8>) {
    let (_, routers) = reflectapi::Builder::new()
        .route(forward, |r| r.name("events.forward"))
        .build()
        .unwrap();
    let dispatcher = reflectapi::dispatch::Dispatcher::new(routers);
    let mut headers = http::HeaderMap::new();
    headers.insert(http::header::CONTENT_TYPE, content_type.parse().unwrap());
    let reply = dispatcher
        .dispatch(
            (),
            &http::Method::POST,
            "/events.forward",
            &headers,
            body.into(),
        )
        .await
        .unwrap();
    match reply {
        reflectapi::dispatch::Reply::Complete { status, body, .. } => (status, body.to_vec()),
        _ => panic!("expected a complete reply"),
    }
}

#[tokio::test]
async fn raw_forwards_msgpack() {
    let event = Event {
        kind: "refund".into(),
        amount: 5,
    };
    let (status, body) = call(
        "application/msgpack",
        rmp_serde::to_vec_named(&event).unwrap(),
    )
    .await;
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(rmp_serde::from_slice::<Event>(&body).unwrap(), event);
}

#[tokio::test]
async fn raw_forwards_json_unchanged() {
    let text = r#"{ "kind": "refund",  "amount": 5, "extra": [1] }"#;
    let (status, body) = call("application/json", text.into()).await;
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(std::str::from_utf8(&body).unwrap(), text);
}
