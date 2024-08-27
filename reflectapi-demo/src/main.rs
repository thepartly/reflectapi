#[tokio::main]
async fn main() {
    let builder = reflectapi_demo::builder();
    let (schema, mut routers) = match builder.build() {
        Ok((schema, routers)) => (schema, routers),
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        }
    };

    let codegen_config = reflectapi::codegen::Config::default();
    let openapi_schema_json: &'static str = Box::leak(
        reflectapi::codegen::openapi::generate(schema.clone(), &codegen_config)
            .unwrap()
            .into_boxed_str(),
    );

    routers.extend([
        reflectapi::Router {
            name: "openapi".to_string(),
            handlers: vec![reflectapi::Handler {
                name: "openapi.json".to_string(),
                path: "".to_string(),
                readonly: true,
                input_headers: vec![],
                callback: std::sync::Arc::new(move |_state, _body| {
                    Box::pin(async move {
                        reflectapi::HandlerOutput {
                            code: http::StatusCode::OK,
                            body: openapi_schema_json.into(),
                            headers: http::HeaderMap::from_iter([(
                                http::HeaderName::from_static("content-type"),
                                "application/json".parse().unwrap(),
                            )]),
                        }
                    })
                }),
            }],
        },
        reflectapi::Router {
            name: "openapi".to_string(),
            handlers: vec![reflectapi::Handler {
                name: "doc".to_string(),
                path: "".to_string(),
                readonly: true,
                input_headers: vec![],
                callback: std::sync::Arc::new(|_state, _body| {
                    Box::pin(async move {
                        reflectapi::HandlerOutput {
                            code: http::StatusCode::OK,
                            body: include_bytes!("./redoc.html")[..].into(),
                            headers: {
                                http::HeaderMap::from_iter([(
                                    http::HeaderName::from_static("content-type"),
                                    "text/html".parse().unwrap(),
                                )])
                            },
                        }
                    })
                }),
            }],
        },
    ]);

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
    eprintln!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, axum_app).await.unwrap();
}
