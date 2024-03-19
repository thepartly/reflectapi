use axum::{
    http::response::Builder,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use crate::{Handler, HandlerInput, HandlerOutput};

pub fn into_axum_app<S>(app_state: S, handlers: Vec<Handler<S>>) -> Router
where
    S: Send + Clone + 'static,
{
    let mut app = Router::new();
    for handler in handlers {
        let Handler {
            name,
            readonly,
            input_headers,
            callback,
        } = handler;
        let axum_handler = {
            let shared_state = app_state.clone();
            move |axum_headers: axum::http::HeaderMap, body: axum::body::Bytes| async move {
                let mut headers = std::collections::HashMap::new();
                for h in input_headers {
                    if let Some(value) = axum_headers.get(&h) {
                        headers.insert(h, value.to_str().unwrap_or_default().to_string());
                    }
                }
                let result = callback(shared_state, HandlerInput { body, headers }).await;
                result.into_response()
            }
        };
        if readonly {
            // Partly API over HTTP standard requires to expose readonly methods on GET and POST
            app = app.route(format!("/{}", name).as_str(), get(axum_handler.clone()));
        }
        app = app.route(format!("/{}", name).as_str(), post(axum_handler));
    }
    app
}

impl IntoResponse for HandlerOutput {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let mut builder = Builder::new().status(self.code);
        for (key, value) in self.headers {
            builder = builder.header(key, value);
        }
        builder.body(self.body.into()).unwrap()
    }
}
