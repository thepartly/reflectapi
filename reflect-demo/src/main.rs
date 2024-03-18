#[cfg(test)]
mod tests;

struct AppState {
    // ...
}

#[tokio::main]
async fn main() {
    let mut schema = reflect_endpoint::SchemaBuilder::new();
    schema.with_function("example", "example function", handler_example, true);
    let (schema, handlers) = schema.build();

    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    let app_state = std::sync::Arc::new(AppState { /* ... */ });
    let axum_app = reflect_axum::into_axum_app(app_state, handlers);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}

#[derive(reflect::Input, serde::Deserialize)]
struct ExampleRequest {}
struct ExampleRequestHeaders {}
#[derive(reflect::Output, serde::Serialize)]
struct ExampleResponse {
    message: String,
}
struct ExampleResponseHeaders {}
#[derive(reflect::Output, serde::Serialize)]
enum ExampleError {
    Error1,
}
impl std::fmt::Display for ExampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error1")
    }
}
impl reflect_endpoint::ToStatusCode for ExampleError {
    fn to_status_code(&self) -> u16 {
        500
    }
}

enum ExampleErrorHeaders {}

async fn handler_example(
    state: std::sync::Arc<AppState>,
    request: ExampleRequest,
) -> Result<ExampleResponse, ExampleError> {
    println!("called");
    // Ok(ExampleResponse {
    //     message: "hello world".to_string(),
    // })
    Err(ExampleError::Error1)
}
