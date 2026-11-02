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
//!
//! A server that partitions its requests by their partition key, the fields
//! marked `#[reflectapi(partition_key = N)]`, finds a request's key with
//! [`Dispatcher::partition_key`] and serves it with [`Dispatcher::dispatch_in`],
//! which checks that the request is the partition's.

use std::{collections::HashMap, fmt, pin::Pin};

use futures_util::Stream;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};

use crate::builder::{HandlerInput, HandlerOutput, PartitionKeyFn};
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
/// adapter and the [`Dispatcher`] both do, checking the request against
/// `partition` first if one is given.
pub(crate) async fn invoke<S>(
    callback: &HandlerCallback<S>,
    input_headers: &[HeaderName],
    state: S,
    headers: &HeaderMap,
    body: bytes::Bytes,
    partition: Option<&[&str]>,
) -> Reply
where
    S: Send + 'static,
{
    let input = handler_input(input_headers, headers, body);
    match callback {
        HandlerCallback::Future(f) => f(state, input, partition).await.into(),
        HandlerCallback::Stream(f) => match f(state, input, partition) {
            Ok(stream) => Reply::Stream(stream),
            Err(output) => output.into(),
        },
    }
}

/// A handler's input: the body, and the subset of `headers` it declared.
fn handler_input(
    input_headers: &[HeaderName],
    headers: &HeaderMap,
    body: bytes::Bytes,
) -> HandlerInput {
    let mut forwarded = HeaderMap::new();
    for name in input_headers {
        if let Some(value) = headers.get(name) {
            forwarded.insert(name.clone(), value.clone());
        }
    }
    HandlerInput {
        body,
        headers: forwarded,
    }
}

struct Route<S>
where
    S: Send + 'static,
{
    readonly: bool,
    input_headers: Vec<HeaderName>,
    callback: HandlerCallback<S>,
    partition_key: Option<PartitionKeyFn>,
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
                partition_key,
            } in router.handlers
            {
                routes.insert(
                    format!("{path}/{name}"),
                    Route {
                        readonly,
                        input_headers,
                        callback,
                        partition_key,
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
        let route = match self.route(method, path)? {
            Ok(route) => route,
            Err(reply) => return Some(reply),
        };
        Some(
            invoke(
                &route.callback,
                &route.input_headers,
                state,
                headers,
                body,
                None,
            )
            .await,
        )
    }

    /// Dispatches one request as [`Dispatcher::dispatch`] does, for the
    /// server of one partition: after parsing the request and before calling
    /// the handler, compares the request's partition key with `partition`,
    /// and answers `400 Bad Request` without calling the handler when they
    /// differ. A route without a partition key is dispatched as by
    /// `dispatch`. The request is parsed once, for the check and the handler
    /// both.
    pub async fn dispatch_in(
        &self,
        partition: &[&str],
        state: S,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        body: bytes::Bytes,
    ) -> Option<Reply> {
        let route = match self.route(method, path)? {
            Ok(route) => route,
            Err(reply) => return Some(reply),
        };
        Some(
            invoke(
                &route.callback,
                &route.input_headers,
                state,
                headers,
                body,
                Some(partition),
            )
            .await,
        )
    }

    /// The partition key of a request: the values of the fields marked
    /// `#[reflectapi(partition_key = N)]` in its route's input and headers
    /// types, in position order, parsed from the body and the headers as the
    /// handler would get them. The handler is not called.
    ///
    /// - `Ok(None)` when the route has no partition key, and when
    ///   [`Dispatcher::dispatch`] would answer without calling a handler: no
    ///   route has the path, or the route does not accept the method;
    /// - `Err` with the reply the handler would give when the request does
    ///   not parse, such as a `400` for a body that is not the input.
    #[allow(clippy::result_large_err)]
    pub fn partition_key(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        body: bytes::Bytes,
    ) -> Result<Option<Vec<String>>, Reply> {
        let Some(Ok(route)) = self.route(method, path) else {
            return Ok(None);
        };
        let Some(partition_key) = &route.partition_key else {
            return Ok(None);
        };
        partition_key(handler_input(&route.input_headers, headers, body))
            .map(Some)
            .map_err(Reply::from)
    }

    /// The route at `path`: `None` if there is none, and the `405` reply if
    /// it does not accept `method`.
    fn route(&self, method: &Method, path: &str) -> Option<Result<&Route<S>, Reply>> {
        let route = self.routes.get(path)?;
        let accepted = *method == Method::POST || (route.readonly && *method == Method::GET);
        if !accepted {
            let mut headers = HeaderMap::new();
            headers.insert(
                http::header::ALLOW,
                HeaderValue::from_static(if route.readonly { "GET,POST" } else { "POST" }),
            );
            return Some(Err(Reply::Complete {
                status: StatusCode::METHOD_NOT_ALLOWED,
                headers,
                body: bytes::Bytes::new(),
            }));
        }
        Some(Ok(route))
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
