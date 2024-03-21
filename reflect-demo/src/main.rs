#[cfg(test)]
mod tests;

struct AppState {
    // ...
}

#[tokio::main]
async fn main() {
    let builder = reflect::Builder::new()
        .name("Demo application".to_string())
        .description("This is a demo application".to_string())
        .route(handler_example, |b| {
            b.name("example".into())
                .readonly(true)
                .description("example description".into())
        })
        .route(handler_example_2, |b| b.name("example2".into()))
        .route(handler_example_3, |b| b.name("example3".into()));
    let (schema, handlers) = match builder.build(vec![("reflect_demo::", "myapi::")], Vec::new()) {
        Ok((schema, handlers)) => (schema, handlers),
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        }
    };

    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    let app_state = std::sync::Arc::new(AppState { /* ... */ });
    let axum_app = reflect::axum::into_router(app_state, handlers);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}

/// Some example doc
/// test
#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct ExampleRequest {
    #[serde(rename = "inputData")]
    input_data: String,

    #[serde(skip_serializing)]
    input_optional: Option<String>,
}

#[derive(reflect::Input, serde::Deserialize)]
struct ExampleRequestHeaders {
    name: String,
}

#[derive(reflect::Output, serde::Serialize)]
struct ExampleResponse {
    /// some doc
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
) -> Result<ExampleRequest, ExampleError> {
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
        message: format!(
            "hello {} -> {} / {}",
            request.input_data,
            headers.name,
            request.input_optional.unwrap_or_default()
        ),
    }
    // Err(ExampleError::Error1)
}
