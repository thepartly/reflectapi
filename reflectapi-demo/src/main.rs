use std::error::Error;

use axum::{response::Html, Json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let builder = reflectapi_demo::builder();
    let (schema, routers) = builder.build()?;
    let openapi_spec = reflectapi::codegen::openapi::Spec::from(&schema);

    // write reflect schema to a file
    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await?;

    // start the server based on axum web framework
    let app_state = Default::default();
    let axum_app = reflectapi::into_router(app_state, routers, |_name, r| {
        // let's append some tracing middleware
        // it can be different depending on the router name,
        // (we have only 1 in the demo example)
        r.layer(tower_http::trace::TraceLayer::new_for_http())
    })
    .route(
        "/openapi.json",
        axum::routing::get(|| async { Json(openapi_spec) }),
    )
    .route(
        "/doc",
        axum::routing::get(|| async { Html(include_str!("./swagger-ui.html")) }),
    )
    .route(
        "/redoc",
        axum::routing::get(|| async { Html(include_str!("./redoc.html")) }),
    );

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    eprintln!("Listening on http://{}", bind_addr);
    axum::serve(listener, axum_app).await?;

    Ok(())
}
