#[cfg(test)]
mod tests;

struct AppState {
    // ...
}

#[tokio::main]
async fn main() {
    let mut schema = reflect::Builder::new();
    // let a = Handler::new("".into(), false, handler_example);
    schema.route("example", "example function", true, handler_example);
    schema.route("example2", "example function2", true, handler_example_2);
    schema.route("example3", "example function2", true, handler_example_3);
    let (schema, handlers) = schema.build();

    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    let app_state = std::sync::Arc::new(AppState { /* ... */ });
    let axum_app = reflect::into_axum_app(app_state, handlers);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}

#[derive(reflect::Input, serde::Deserialize)]
struct ExampleRequest {
    #[serde(rename = "inputData")]
    input_data: String,
}

#[derive(reflect::Input, serde::Deserialize)]
struct ExampleRequestHeaders {
    name: String,
}
#[derive(reflect::Output, serde::Serialize)]
struct ExampleResponse {
    message: String,
}
#[derive(reflect::Output, serde::Serialize)]
enum ExampleError {
    Error1,
}

impl reflect::StatusCode for ExampleError {
    fn status_code(&self) -> u16 {
        axum::http::StatusCode::UNPROCESSABLE_ENTITY.as_u16()
    }
}

async fn handler_example(
    state: std::sync::Arc<AppState>,
    request: ExampleRequest,
    headers: ExampleRequestHeaders,
) -> Result<ExampleResponse, ExampleError> {
    println!("called");
    // Ok(ExampleResponse {
    //     message: format!("hello {}", request.input_data),
    // })
    Err(ExampleError::Error1)
}

async fn handler_example_3(
    state: std::sync::Arc<AppState>,
    request: reflect::Empty,
    headers: reflect::Empty,
) -> reflect::Empty {
    println!("called");
    // Ok(ExampleResponse {
    //     message: format!("hello {}", request.input_data),
    // })
    // Err(ExampleError::Error1)

    // Default::default()
    ().into()
}

async fn handler_example_2(
    state: std::sync::Arc<AppState>,
    request: ExampleRequest,
    headers: ExampleRequestHeaders,
) -> ExampleResponse {
    println!("called");
    ExampleResponse {
        message: format!("hello {} -> {}", request.input_data, headers.name),
    }
    // Err(ExampleError::Error1)
}
