use axum::{
    http::response::Builder,
    response::{sse, IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;

use crate::{
    builder::{HandlerInput, HandlerOutput},
    Handler, HandlerCallback,
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
                let mut headers = http::HeaderMap::new();
                for h in input_headers {
                    if let Some(value) = axum_headers.get(&h) {
                        headers.insert(h, value.clone());
                    }
                }
                let input = HandlerInput { body, headers };
                match callback {
                    HandlerCallback::Future(f) => f(state, input).await.into_response(),
                    HandlerCallback::Stream(f) => match f(state, input) {
                        Ok(st) => {
                            Sse::new(st.map(|s| s.map(|data| sse::Event::default().data(data))))
                                .into_response()
                        }
                        Err(err) => err.into_response(),
                    },
                }
            }
        };
        let mount_path = format!("{}/{}", path, name);
        if readonly {
            // Partly API over HTTP standard requires to expose readonly methods on GET and POST
            app = app.route(mount_path.as_str(), get(axum_handler.clone()));
        }
        app = app.route(mount_path.as_str(), post(axum_handler));
    }
    (name, app)
}

impl IntoResponse for HandlerOutput {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let mut builder = Builder::new().status(self.code);
        *builder.headers_mut().unwrap() = self.headers;
        builder.body(self.body.into()).unwrap()
    }
}
