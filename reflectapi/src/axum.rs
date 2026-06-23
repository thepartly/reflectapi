use axum::{
    http::response::Builder,
    response::{sse, IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;

use crate::{
    builder::{HandlerInput, HandlerOutput},
    retry::RetryBudget,
    Handler, HandlerCallback,
};

/// Build an axum router from the reflectapi routes without server-side retry.
pub fn into_router<S, F>(app_state: S, router: Vec<crate::Router<S>>, cb: F) -> Router
where
    S: Send + Clone + Sync + 'static,
    F: Fn(String, Router) -> Router,
{
    let mut app = Router::new();
    for r in router {
        let (name, router) = into_router_one(app_state.clone(), r, None);
        let router = cb(name, router);
        app = app.merge(router);
    }
    app
}

/// Build an axum router with server-side retry budget for retriable endpoints.
///
/// Retriable endpoints that return a retryable status code will be automatically
/// retried (up to `max_retries_per_request` times) as long as the shared retry
/// budget has not been exhausted.
pub fn into_router_with_retries<S, F>(
    app_state: S,
    router: Vec<crate::Router<S>>,
    budget: RetryBudget,
    cb: F,
) -> Router
where
    S: Send + Clone + Sync + 'static,
    F: Fn(String, Router) -> Router,
{
    let mut app = Router::new();
    for r in router {
        let (name, router) = into_router_one(app_state.clone(), r, Some(budget.clone()));
        let router = cb(name, router);
        app = app.merge(router);
    }
    app
}

fn into_router_one<S>(
    app_state: S,
    router: crate::Router<S>,
    budget: Option<RetryBudget>,
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
            retriable,
            input_headers,
            callback,
        } = handler;
        let budget = budget.clone();
        let axum_handler = {
            let state = app_state.clone();
            move |axum_headers: http::HeaderMap, body: axum::body::Bytes| async move {
                let mut headers = http::HeaderMap::new();
                for h in &input_headers {
                    if let Some(value) = axum_headers.get(h) {
                        headers.insert(h.clone(), value.clone());
                    }
                }
                let input = HandlerInput {
                    body: body.clone(),
                    headers: headers.clone(),
                };

                match &callback {
                    HandlerCallback::Future(f) => {
                        if let Some(ref budget) = budget {
                            if retriable {
                                budget.record_request();
                            }
                        }

                        let response = f(state.clone(), input).await;
                        let status = response.code.as_u16();

                        // Retry if retriable, budget allows, and status is retryable
                        if retriable {
                            if let Some(ref budget) = budget {
                                if budget.should_retry_status(status) {
                                    let mut attempts = 0;
                                    let max = budget.max_retries_per_request();
                                    let mut last_response = response;

                                    while attempts < max && budget.try_acquire_retry() {
                                        attempts += 1;
                                        let retry_input = HandlerInput {
                                            body: body.clone(),
                                            headers: headers.clone(),
                                        };
                                        last_response = f(state.clone(), retry_input).await;
                                        if !budget
                                            .should_retry_status(last_response.code.as_u16())
                                        {
                                            break;
                                        }
                                    }
                                    return last_response.into_response();
                                }
                            }
                        }

                        response.into_response()
                    }
                    HandlerCallback::Stream(f) => {
                        let input = HandlerInput { body, headers };
                        match f(state, input) {
                            Ok(st) => Sse::new(
                                st.map(|s| s.map(|data| sse::Event::default().data(data))),
                            )
                            .into_response(),
                            Err(err) => err.into_response(),
                        }
                    }
                }
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

impl IntoResponse for HandlerOutput {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let mut builder = Builder::new().status(self.code);
        *builder.headers_mut().unwrap() = self.headers;
        builder.body(self.body.into()).unwrap()
    }
}
