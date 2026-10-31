use futures_util::StreamExt as _;
use std::time::Duration;
use tower::Service as _;

fn delayed_event(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<impl futures_util::Stream<Item = String>, reflectapi::Infallible> {
    Ok(futures_util::stream::once(async {
        tokio::time::sleep(Duration::from_secs(16)).await;
        "next".to_owned()
    }))
}

#[tokio::test(start_paused = true)]
async fn idle_stream_sends_keepalive_without_changing_data_or_termination(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, routers) = reflectapi::Builder::default()
        .stream_route(delayed_event, |route| route.name("delayed"))
        .build()?;
    let mut app = reflectapi::axum::into_router((), routers, |_, router| router);
    let response = app
        .call(
            http::Request::builder()
                .method(http::Method::POST)
                .uri("/delayed")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.headers()[http::header::CONTENT_TYPE],
        "text/event-stream"
    );
    let mut frames = response.into_body().into_data_stream();
    let keepalive = tokio::time::timeout(Duration::from_secs(16), frames.next())
        .await?
        .ok_or("stream ended before keepalive")??;
    assert_eq!(keepalive.as_ref(), b":\n\n");
    let event = tokio::time::timeout(Duration::from_secs(2), frames.next())
        .await?
        .ok_or("stream ended before event")??;
    assert_eq!(event.as_ref(), b"data: \"next\"\n\n");
    assert!(frames.next().await.is_none());
    Ok(())
}
