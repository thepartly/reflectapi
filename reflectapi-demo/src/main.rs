#[tokio::main]
async fn main() {
    let builder = reflectapi_demo::builder();
    let (schema, routers) = match builder.build() {
        Ok((schema, routers)) => (schema, routers),
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        }
    };

    // write reflect schema to a file
    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    // start the server based on axum web framework
    let app_state = Default::default();
    let axum_app = reflectapi::axum::into_router(app_state, routers, |_name, r| {
        // let's append some tracing middleware
        // it can be different depending on the router name,
        // (we have only 1 in the demo example)
        r.layer(tower_http::trace::TraceLayer::new_for_http())
    });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}
