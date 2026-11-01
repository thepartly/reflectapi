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
//!     Some(Reply::Stream(events)) => respond_sse(sse_frames(events)),
//! }
//! ```
//!
//! Proxies close a connection that sends nothing for a while, so a stream
//! route that is quiet for long periods can wrap its frames in
//! [`keep_alive`] with the host runtime's timer.

use std::{collections::HashMap, fmt, future::Future, pin::Pin, time::Duration};

use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt};
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

/// Frames each item of a [`Reply::Stream`] with [`sse_event`]: the body of a
/// `text/event-stream` response, before any [`keep_alive`].
pub fn sse_frames(events: ReplyStream) -> impl Stream<Item = Result<String, String>> + Send {
    events.map(|item| item.map(|event| sse_event(&event)))
}

/// The comment [`keep_alive`] writes: a comment line, then a blank line.
/// Clients that follow the SSE spec ignore it.
const KEEP_ALIVE_COMMENT: &str = ":\n\n";

/// Writes an SSE comment whenever `frames` has written nothing for
/// `interval`, so that proxies and load balancers that close idle
/// connections keep a quiet stream open.
///
/// The interval restarts after every event and every comment, so a busy
/// stream carries no extra bytes. Nothing is written after the stream ends
/// or fails. A zero interval writes no comments: a timer that is always due
/// would otherwise flood the connection with them. `sleep` is the host runtime's timer, such as
/// `tokio::time::sleep`; the result is `Send` exactly when `sleep` and its
/// futures are, so a single-threaded runtime's timer works too.
///
/// ```rust,ignore
/// let body = keep_alive(sse_frames(events), Duration::from_secs(15), tokio::time::sleep);
/// ```
pub fn keep_alive<S, E, F, Fut>(
    frames: S,
    interval: Duration,
    sleep: F,
) -> impl Stream<Item = Result<String, E>>
where
    S: Stream<Item = Result<String, E>> + Unpin,
    F: FnMut(Duration) -> Fut,
    Fut: Future<Output = ()>,
{
    enum Next<E> {
        Frame(Result<String, E>),
        Idle,
        End,
    }

    futures_util::stream::unfold(Some((frames, sleep)), move |state| async move {
        let (mut frames, mut sleep) = state?;
        let next = if interval.is_zero() {
            match frames.next().await {
                Some(frame) => Next::Frame(frame),
                None => Next::End,
            }
        } else {
            let timer = sleep(interval);
            futures_util::pin_mut!(timer);
            // Dropping a pending `next()` loses nothing: a stream yields an
            // item only from the poll that returns it.
            match future::select(frames.next(), timer).await {
                Either::Left((Some(frame), _)) => Next::Frame(frame),
                Either::Left((None, _)) => Next::End,
                Either::Right(((), _)) => Next::Idle,
            }
        };
        match next {
            Next::Frame(Ok(frame)) => Some((Ok(frame), Some((frames, sleep)))),
            // The response ends with an error, so nothing follows it.
            Next::Frame(Err(err)) => Some((Err(err), None)),
            Next::Idle => Some((Ok(KEEP_ALIVE_COMMENT.to_owned()), Some((frames, sleep)))),
            Next::End => None,
        }
    })
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

    mod keep_alive {
        use std::{cell::RefCell, future::Future, rc::Rc, time::Duration};

        use futures::channel::{mpsc, oneshot};
        use futures::{FutureExt, Stream, StreamExt};

        type Frames = mpsc::UnboundedSender<Result<String, String>>;

        /// Timers the test fires by hand, newest last.
        #[derive(Clone, Default)]
        struct Timers(Rc<RefCell<Vec<oneshot::Sender<()>>>>);

        impl Timers {
            fn sleep(&self) -> impl FnMut(Duration) -> std::pin::Pin<Box<dyn Future<Output = ()>>> {
                let timers = self.clone();
                move |interval| {
                    assert_eq!(interval, Duration::from_secs(15));
                    let (fire, fired) = oneshot::channel();
                    timers.0.borrow_mut().push(fire);
                    Box::pin(fired.map(|_| ()))
                }
            }

            fn started(&self) -> usize {
                self.0.borrow().len()
            }

            fn fire(&self, index: usize) {
                let fire = std::mem::replace(&mut self.0.borrow_mut()[index], oneshot::channel().0);
                let _ = fire.send(());
            }
        }

        fn kept_alive(
            timers: &Timers,
        ) -> (Frames, impl Stream<Item = Result<String, String>> + Unpin) {
            let (frames, rx) = mpsc::unbounded();
            let stream = Box::pin(super::super::keep_alive(
                rx,
                Duration::from_secs(15),
                timers.sleep(),
            ));
            (frames, stream)
        }

        #[test]
        fn a_zero_interval_writes_no_comments() {
            let (frames, rx) = mpsc::unbounded::<Result<String, String>>();
            // A timer that is always due, as `tokio::time::sleep(Duration::ZERO)` is.
            let mut stream = Box::pin(super::super::keep_alive(rx, Duration::ZERO, |_| {
                futures::future::ready(())
            }));

            assert_eq!(stream.next().now_or_never(), None);
            frames.unbounded_send(Ok("a".into())).unwrap();
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok("a".into()))));
            assert_eq!(stream.next().now_or_never(), None);
        }

        #[test]
        fn an_idle_stream_writes_a_comment_each_interval() {
            let timers = Timers::default();
            let (_frames, mut stream) = kept_alive(&timers);

            assert_eq!(stream.next().now_or_never(), None);
            timers.fire(0);
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok(":\n\n".into()))));
            assert_eq!(stream.next().now_or_never(), None);
            assert_eq!(timers.started(), 2);
            timers.fire(1);
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok(":\n\n".into()))));
        }

        #[test]
        fn each_frame_restarts_the_interval() {
            let timers = Timers::default();
            let (frames, mut stream) = kept_alive(&timers);

            assert_eq!(stream.next().now_or_never(), None);
            frames.unbounded_send(Ok("a".into())).unwrap();
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok("a".into()))));
            frames.unbounded_send(Ok("b".into())).unwrap();
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok("b".into()))));
            assert_eq!(stream.next().now_or_never(), None);
            // The timers started before "a" and "b" no longer count.
            timers.fire(0);
            timers.fire(1);
            assert_eq!(stream.next().now_or_never(), None);
            timers.fire(2);
            assert_eq!(stream.next().now_or_never(), Some(Some(Ok(":\n\n".into()))));
        }

        #[test]
        fn nothing_is_written_after_the_end() {
            let timers = Timers::default();
            let (frames, mut stream) = kept_alive(&timers);

            assert_eq!(stream.next().now_or_never(), None);
            drop(frames);
            timers.fire(0);
            assert_eq!(stream.next().now_or_never(), Some(None));
        }

        #[test]
        fn nothing_is_written_after_an_error() {
            let timers = Timers::default();
            let (frames, mut stream) = kept_alive(&timers);

            frames.unbounded_send(Err("boom".into())).unwrap();
            frames.unbounded_send(Ok("after".into())).unwrap();
            assert_eq!(stream.next().now_or_never(), Some(Some(Err("boom".into()))));
            assert_eq!(stream.next().now_or_never(), Some(None));
            assert_eq!(timers.started(), 1);
        }
    }
}
