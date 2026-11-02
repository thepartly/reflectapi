use std::{fmt, future::Future, pin::Pin, sync::Arc, time::Duration};

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use crate::{
    builder::HandlerOutput,
    dispatch::{invoke, keep_alive, sse_frames, Reply},
    Handler,
};

type Sleep = Arc<dyn Fn(Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// How [`into_router_with`] serves the routes. The default is what
/// [`into_router`] does.
///
/// These are serving settings, not part of the API: the right keep-alive
/// interval depends on the proxies in front of a deployment, and the same
/// API definition builds the schema and may be served in several places.
/// The server-wide default is set here; a stream route can override it with
/// [`crate::RouteBuilder::sse_keep_alive`].
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ServeOptions {
    keep_alive: Option<Duration>,
    timer: Option<Sleep>,
}

impl ServeOptions {
    /// Writes an SSE comment on a stream route that has sent nothing for
    /// `interval`, using `sleep` as the timer, such as `tokio::time::sleep`.
    /// A zero interval writes none. A route's own
    /// [`crate::RouteBuilder::sse_keep_alive`] takes precedence. See
    /// [`crate::dispatch::keep_alive`].
    ///
    /// ```rust,ignore
    /// let options = ServeOptions::default()
    ///     .sse_keep_alive(Duration::from_secs(15), tokio::time::sleep);
    /// ```
    pub fn sse_keep_alive<F, Fut>(self, interval: Duration, sleep: F) -> Self
    where
        F: Fn(Duration) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            keep_alive: Some(interval),
            ..self.sse_timer(sleep)
        }
    }

    /// The timer for SSE keep-alive, such as `tokio::time::sleep`, without a
    /// server-wide interval: only stream routes that set
    /// [`crate::RouteBuilder::sse_keep_alive`] write comments.
    pub fn sse_timer<F, Fut>(mut self, sleep: F) -> Self
    where
        F: Fn(Duration) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.timer = Some(Arc::new(move |interval| Box::pin(sleep(interval))));
        self
    }
}

impl fmt::Debug for ServeOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServeOptions")
            .field("sse_keep_alive", &self.keep_alive)
            .field("sse_timer", &self.timer.is_some())
            .finish()
    }
}

pub fn into_router<S, F>(app_state: S, router: Vec<crate::Router<S>>, cb: F) -> Router
where
    S: Send + Clone + Sync + 'static,
    F: Fn(String, Router) -> Router,
{
    into_router_with(app_state, router, ServeOptions::default(), cb)
}

/// [`into_router`], served as `options` says.
///
/// # Panics
///
/// If a stream route sets a non-zero
/// [`crate::RouteBuilder::sse_keep_alive`] and `options` has no timer.
pub fn into_router_with<S, F>(
    app_state: S,
    router: Vec<crate::Router<S>>,
    options: ServeOptions,
    cb: F,
) -> Router
where
    S: Send + Clone + Sync + 'static,
    F: Fn(String, Router) -> Router,
{
    let mut app = Router::new();
    for r in router {
        let (name, router) = into_router_one(app_state.clone(), r, &options);
        let router = cb(name, router);
        app = app.merge(router);
    }
    app
}

fn into_router_one<S>(
    app_state: S,
    router: crate::Router<S>,
    options: &ServeOptions,
) -> (String, Router)
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
            sse_keep_alive,
            callback,
        } = handler;
        // The route's own interval wins over the server's; zero means none.
        let keep_alive = match sse_keep_alive.or(options.keep_alive) {
            Some(interval) if !interval.is_zero() => {
                let Some(sleep) = options.timer.clone() else {
                    panic!(
                        "route `{path}/{name}` sets an SSE keep-alive interval, but the \
                         ServeOptions have no timer: add one with `sse_timer` or `sse_keep_alive`"
                    );
                };
                Some((interval, sleep))
            }
            _ => None,
        };
        let axum_handler = {
            let state = app_state.clone();
            let keep_alive = keep_alive.clone();
            move |axum_headers: http::HeaderMap, body: axum::body::Bytes| async move {
                let reply = invoke(&callback, &input_headers, state, &axum_headers, body).await;
                into_response(reply, keep_alive)
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
        into_response(self, None)
    }
}

fn into_response(
    reply: Reply,
    keep_alive_after: Option<(Duration, Sleep)>,
) -> axum::response::Response {
    match reply {
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
        Reply::Stream(stream) => {
            let frames = sse_frames(stream);
            let body = match keep_alive_after {
                Some((interval, sleep)) => {
                    axum::body::Body::from_stream(keep_alive(frames, interval, move |interval| {
                        sleep(interval)
                    }))
                }
                None => axum::body::Body::from_stream(frames),
            };
            (
                [
                    (http::header::CONTENT_TYPE, "text/event-stream"),
                    (http::header::CACHE_CONTROL, "no-cache"),
                ],
                body,
            )
                .into_response()
        }
    }
}

impl IntoResponse for HandlerOutput {
    fn into_response(self) -> axum::response::Response {
        Reply::from(self).into_response()
    }
}
