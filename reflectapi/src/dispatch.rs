//! Request dispatch that is independent of any web framework.
//!
//! Use it where the runtime brings its own HTTP server and pulling in axum is
//! not wanted, such as Cloudflare-compatible Workers (Cloudflare, celld). The
//! host adapter turns its request into [`Dispatcher::dispatch`] arguments and
//! the returned [`Reply`] into its response:
//!
//! ```rust,ignore
//! let routers = api::<reflectapi::DispatchOnly>().build()?;
//! let dispatcher = reflectapi::dispatch::Dispatcher::new(routers);
//!
//! match dispatcher.dispatch(state, &method, &path, &headers, body).await {
//!     None => not_found(),
//!     Some(Reply::Complete { status, headers, body }) => respond(status, headers, body),
//!     Some(Reply::Stream(events)) => respond_sse(events.map(|event| event.map(|e| sse_event(&e)))),
//! }
//! ```

use std::{collections::HashMap, fmt, pin::Pin};

use futures_util::Stream;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};

use crate::builder::{HandlerInput, HandlerOutput};
use crate::{Event, Handler, HandlerCallback, Router};

/// The outcome of dispatching one request.
pub enum Reply {
    /// A complete response.
    Complete {
        status: StatusCode,
        headers: HeaderMap,
        body: bytes::Bytes,
    },
    /// The response of a stream route, to send as `text/event-stream`.
    /// Each `Ok` item is one event: a serialized value, with the event id the
    /// route gave it, if any (see [`sse_event`]). An `Err` item reports a
    /// value that failed to serialize; the response ends there.
    Stream(ReplyStream),
}

/// The items of a [`Reply::Stream`].
pub type ReplyStream = Pin<Box<dyn Stream<Item = Result<Event<String>, String>> + Send + 'static>>;

impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reply::Complete {
                status,
                headers,
                body,
            } => f
                .debug_struct("Complete")
                .field("status", status)
                .field("headers", headers)
                .field("body", body)
                .finish(),
            Reply::Stream(_) => f.debug_tuple("Stream").finish_non_exhaustive(),
        }
    }
}

impl From<HandlerOutput> for Reply {
    fn from(output: HandlerOutput) -> Self {
        Reply::Complete {
            status: output.code,
            headers: output.headers,
            body: output.body,
        }
    }
}

/// Frames one [`Reply::Stream`] item as a Server-Sent Event: an `id:` line if
/// it has an id, one `data:` line per line of its data, then a blank line.
pub fn sse_event(Event { id, data }: &Event<String>) -> String {
    let mut event = String::with_capacity(data.len() + 8);
    if let Some(id) = id {
        event.push_str("id: ");
        event.push_str(id.as_str());
        event.push('\n');
    }
    for line in data.split('\n') {
        event.push_str("data: ");
        event.push_str(line.strip_suffix('\r').unwrap_or(line));
        event.push('\n');
    }
    event.push('\n');
    event
}

/// Calls a handler with the subset of `headers` it declared, as the axum
/// adapter and the [`Dispatcher`] both do.
pub(crate) async fn invoke<S>(
    callback: &HandlerCallback<S>,
    input_headers: &[HeaderName],
    state: S,
    headers: &HeaderMap,
    body: bytes::Bytes,
) -> Reply
where
    S: Send + 'static,
{
    let mut forwarded = HeaderMap::new();
    for name in input_headers {
        if let Some(value) = headers.get(name) {
            forwarded.insert(name.clone(), value.clone());
        }
    }
    let input = HandlerInput {
        body,
        headers: forwarded,
    };
    match callback {
        HandlerCallback::Future(f) => f(state, input).await.into(),
        HandlerCallback::Stream(f) => match f(state, input) {
            Ok(stream) => Reply::Stream(stream),
            Err(output) => output.into(),
        },
    }
}

struct Route<S>
where
    S: Send + 'static,
{
    readonly: bool,
    input_headers: Vec<HeaderName>,
    callback: HandlerCallback<S>,
}

/// Routes requests to the handlers of built [`Router`]s, by path and method,
/// exactly as `reflectapi::axum::into_router` mounts them: every route accepts
/// `POST`, and readonly routes accept `GET` as well.
pub struct Dispatcher<S>
where
    S: Send + 'static,
{
    routes: HashMap<String, Route<S>>,
}

impl<S> fmt::Debug for Dispatcher<S>
where
    S: Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut paths: Vec<_> = self.routes.keys().collect();
        paths.sort();
        f.debug_struct("Dispatcher")
            .field("routes", &paths)
            .finish()
    }
}

impl<S> Dispatcher<S>
where
    S: Send + 'static,
{
    pub fn new(routers: Vec<Router<S>>) -> Self {
        let mut routes = HashMap::new();
        for router in routers {
            for Handler {
                name,
                path,
                readonly,
                input_headers,
                callback,
            } in router.handlers
            {
                routes.insert(
                    format!("{path}/{name}"),
                    Route {
                        readonly,
                        input_headers,
                        callback,
                    },
                );
            }
        }
        Self { routes }
    }

    /// Dispatches one request, as axum serves the routes `into_router` mounts:
    ///
    /// - `None` when no route has this `path`;
    /// - a `405 Method Not Allowed` reply, with an `allow` header, when the
    ///   route does not accept `method`.
    ///
    /// Unlike axum, it does not answer `HEAD` by running the `GET` handler and
    /// dropping the body: `HEAD` gets `405` like any other unaccepted method.
    pub async fn dispatch(
        &self,
        state: S,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        body: bytes::Bytes,
    ) -> Option<Reply> {
        let route = self.routes.get(path)?;
        let accepted = *method == Method::POST || (route.readonly && *method == Method::GET);
        if !accepted {
            let mut headers = HeaderMap::new();
            headers.insert(
                http::header::ALLOW,
                HeaderValue::from_static(if route.readonly { "GET,POST" } else { "POST" }),
            );
            return Some(Reply::Complete {
                status: StatusCode::METHOD_NOT_ALLOWED,
                headers,
                body: bytes::Bytes::new(),
            });
        }
        Some(invoke(&route.callback, &route.input_headers, state, headers, body).await)
    }
}

#[cfg(test)]
mod tests {
    use super::sse_event;
    use crate::{Event, EventId};

    fn data(data: &str) -> Event<String> {
        Event {
            id: None,
            data: data.into(),
        }
    }

    #[test]
    fn sse_event_frames_each_line() {
        assert_eq!(sse_event(&data(r#"{"a":1}"#)), "data: {\"a\":1}\n\n");
        assert_eq!(
            sse_event(&data("a\nb\r\nc")),
            "data: a\ndata: b\ndata: c\n\n"
        );
        assert_eq!(sse_event(&data("")), "data: \n\n");
    }

    #[test]
    fn sse_event_writes_the_id_first() -> Result<(), crate::InvalidEventId> {
        assert_eq!(
            sse_event(&Event::new(42, "a\nb".to_string())),
            "id: 42\ndata: a\ndata: b\n\n"
        );
        assert_eq!(
            sse_event(&Event::new(EventId::new("")?, "{}".to_string())),
            "id: \ndata: {}\n\n"
        );
        Ok(())
    }
}
