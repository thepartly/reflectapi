use axum::{
    http::response::Builder,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use crate::{Handler, HandlerError, HandlerInput, HandlerOutput};

pub fn into_axum_app<S>(app_state: S, handlers: Vec<Handler<S>>) -> Router
where
    S: Send + Clone + 'static,
{
    let mut app = Router::new();
    for handler in handlers {
        let Handler {
            name,
            readonly,
            required_input_headers: input_headers,
            callback,
        } = handler;
        let axum_handler2 = {
            let shared_state = app_state.clone();
            move |axum_headers: axum::http::HeaderMap, body: axum::body::Bytes| async move {
                let mut headers = std::collections::HashMap::new();
                for h in input_headers {
                    if let Some(value) = axum_headers.get(&h) {
                        headers.insert(h, value.to_str().unwrap_or_default().to_string());
                    }
                }
                let result = callback(shared_state, HandlerInput { body, headers }).await;
                HandlerResultWrap(result).into_response()
            }
        };
        if readonly {
            // Partly API over HTTP standard requires to expose readonly methods on GET and POST
            app = app.route(format!("/{}", name).as_str(), get(axum_handler2.clone()));
        }
        app = app.route(format!("/{}", name).as_str(), post(axum_handler2));
    }
    app
}

struct HandlerResultWrap(Result<HandlerOutput, HandlerError>);

impl IntoResponse for HandlerResultWrap {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        match self.0 {
            Ok(response) => {
                let mut builder = Builder::new().status(200);
                for (key, value) in response.headers {
                    builder = builder.header(key, value);
                }
                builder.body(response.body.into()).unwrap()
            }
            Err(error) => {
                let mut builder = Builder::new().status(error.code);
                for (key, value) in error.headers {
                    builder = builder.header(key, value);
                }
                builder.body(error.body.into()).unwrap()
            }
        }
    }
}
