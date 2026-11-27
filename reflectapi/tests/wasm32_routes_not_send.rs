//! On wasm32, a route's handler and a stream route's stream need not be
//! `Send` (see `reflectapi::MaybeSend`). A wasm32 program runs on one thread,
//! and a call through a generated client there isn't `Send`, so a route that
//! awaits one must still build. CI builds this file for wasm32; natively it
//! is empty, as the builder requires `Send` there.
#![cfg(target_arch = "wasm32")]

use std::rc::Rc;

use futures_util::StreamExt;

/// Holds an `Rc` across an await, as a call through a generated client holds
/// its response's body.
async fn held(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, reflectapi::Infallible> {
    let held = Rc::new(());
    std::future::ready(()).await;
    drop(held);
    Ok(reflectapi::Empty {})
}

/// Returns a stream that holds an `Rc`, as a stream read through a generated
/// client does.
async fn held_stream(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> Result<impl futures_util::Stream<Item = reflectapi::Empty>, reflectapi::Infallible> {
    let held = Rc::new(());
    std::future::ready(()).await;
    Ok(
        futures_util::stream::iter([reflectapi::Empty {}]).map(move |item| {
            let _ = &held;
            item
        }),
    )
}

#[test]
fn routes_need_not_be_send() {
    reflectapi::Builder::<(), reflectapi::DispatchOnly>::default()
        .route(held, |b| b.name("held"))
        .stream_route(held_stream, |b| b.name("held-stream"))
        .build()
        .expect("the API builds");
}
