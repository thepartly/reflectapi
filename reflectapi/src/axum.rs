use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;

use crate::{
    builder::HandlerOutput,
    dispatch::{invoke, sse_event, Reply},
    Handler,
};

pub fn into_router<S, F>(app_state: S, router: Vec<crate::Router<S>>, cb: F) -> Router
where
    S: Send + Clone + Sync + 'static,
    F: Fn(String, Router) -> Router,
{
    let mut app = Router::new();
    for r in router {
        let (name, router) = into_router_one(app_state.clone(), r);
        let router = cb(name, router);
        app = app.merge(router);
    }
    app
}

fn into_router_one<S>(app_state: S, router: crate::Router<S>) -> (String, Router)
where
    S: Send + Clone + Sync + 'static,
{
    let mut app = Router::new();
    let crate::Router { name, handlers } = router;
    for handler in handlers {
        let Handler {
            name,
            path,
            readonly,
            input_headers,
            callback,
        } = handler;
        let axum_handler = {
            let state = app_state.clone();
            move |axum_headers: http::HeaderMap, body: axum::body::Bytes| async move {
                invoke(&callback, &input_headers, state, &axum_headers, body)
                    .await
                    .into_response()
            }
        };
        let mount_path = format!("{path}/{name}");
        if readonly {
            // Partly API over HTTP standard requires to expose readonly methods on GET and POST
            app = app.route(mount_path.as_str(), get(axum_handler.clone()));
        }
        app = app.route(mount_path.as_str(), post(axum_handler));
    }
    (name, app)
}

impl IntoResponse for Reply {
    fn into_response(self) -> axum::response::Response {
        match self {
            Reply::Complete {
                status,
                headers,
                body,
            } => {
                let mut response = axum::response::Response::new(body.into());
                *response.status_mut() = status;
                *response.headers_mut() = headers;
                response
            }
            Reply::Stream(stream) => (
                [
                    (http::header::CONTENT_TYPE, "text/event-stream"),
                    (http::header::CACHE_CONTROL, "no-cache"),
                ],
                axum::body::Body::from_stream(
                    stream.map(|item| item.map(|event| sse_event(&event))),
                ),
            )
                .into_response(),
        }
    }
}

impl IntoResponse for HandlerOutput {
    fn into_response(self) -> axum::response::Response {
        Reply::from(self).into_response()
    }
}
