#[cfg(test)]
mod tests;

use axum::handler::Handler;
use axum::http::response::Builder;
use axum::response::IntoResponse;
use bytes::{Buf, Bytes};
use reflect_endpoint::{HandlerRequest, SchemaWithHandlers};
use std::io::Read;

use std::{collections::HashMap, sync::Arc};

use axum::{routing::get, Router};
use reflect::HeaderName;

struct AppState {
    // ...
}

#[tokio::main]
async fn main() {
    let mut schema = SchemaWithHandlers::new();
    schema.with_function("/example", "example function", handler_example);
    let (schema, handlers) = schema.build();

    let shared_state = Arc::new(AppState { /* ... */ });

    // build our application with a single route
    let mut app = Router::new();
    for (name, handler) in handlers {
        let handler_call = handler.handler();
        app = app.route(
            name.as_str(),
            get({
                let shared_state = Arc::clone(&shared_state);
                let handler2 = Arc::clone(&handler_call);
                move |body: axum::body::Bytes| async move {
                    let result = handler2(
                        shared_state,
                        HandlerRequest {
                            body: body,
                            headers: HashMap::new(),
                        },
                    )
                    .await;

                    match result {
                        Ok(response) => {
                            HandlerResponseWrap(response).into_response()
                            // unimplemented!()
                            // vec![0u8]
                        }
                        Err(error) => {
                            HandlerErrorWrap(error).into_response()
                            // error.body
                            // axum::http::Response::new(error.body)
                        }
                    }
                }
            }),
        );
    }
    // app.route(
    //     "/",
    //     get({
    //         let shared_state = Arc::clone(&shared_state);
    //         move |body: axum::body::Bytes| {
    //             // let reader = body.reader();

    //             // handler_example(shared_state)
    //             ""
    //         }
    //     }),
    // );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

struct HandlerResponseWrap(reflect_endpoint::HandlerResponse);

impl IntoResponse for HandlerResponseWrap {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        Builder::new().status(200).body(self.0.body.into()).unwrap()
    }
}

struct HandlerErrorWrap(reflect_endpoint::HandlerError);

impl IntoResponse for HandlerErrorWrap {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        Builder::new()
            .status(self.0.status)
            .body(self.0.body.into())
            .unwrap()
    }
}

// async fn axum_handler<S: Send>(
//     shared_state: S,
//     handler: Arc<
//         dyn Fn(
//                 S,
//                 HandlerRequest,
//             ) -> std::pin::Pin<
//                 Box<
//                     dyn std::future::Future<
//                             Output = Result<
//                                 reflect_endpoint::HandlerResponse,
//                                 reflect_endpoint::HandlerError,
//                             >,
//                         > + Send,
//                 >,
//             > + Send
//             + Sync,
//     >,
// ) -> impl IntoResponse {
//     move |body: axum::body::Bytes| async move {
//         let result = handler(
//             shared_state,
//             HandlerRequest {
//                 body: body,
//                 headers: HashMap::new(),
//             },
//         )
//         .await;

//         axum::http::Response::new("test")
//         // unimplemented!("handler.call")
//         // match result {
//         //     Ok(response) => {
//         //         unimplemented!()
//         //         // axum::http::Response::new("test")
//         //     }
//         //     Err(error) => {
//         //         unimplemented!()
//         //         // axum::http::Response::new(error.body)
//         //     }
//         // }
//     }
// }

fn test_handler() -> axum::http::Response<Vec<u8>> {
    axum::http::Response::new(vec![0])
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
enum ExampleErrorHeaders {}

async fn handler_example(
    state: Arc<AppState>,
    request: ExampleRequest,
) -> Result<ExampleResponse, ExampleError> {
    println!("called");
    // Ok(ExampleResponse {
    //     message: "hello world".to_string(),
    // })
    Err(ExampleError::Error1)
}

// #[derive(serde::Serialize)]
// struct A {
//     _a: std::marker::PhantomData<MyStruct<u8>>,
// }

// struct MyStruct<T> {
//     f2: T,
//     f: Box<MyStruct<T>>,
// }

// #[derive(reflect::Input)]
// struct TestStructWithVec<T>
// where
//     T: reflect::Input,
// {
//     _f: T,
// }

// #[derive(reflect::Input)]
// struct TestStructParent
// // where
// //     T: reflect::Input,
// {
//     _f: TestStructWithVec<u8>,
//     // _f2: TestStructWithVec<T>,
// }

// #[derive(serde::Deserialize)]
// struct Test<'a> {
//     _a: std::slice::Iter<'a, u8>,
// }

// trait MyTrait {}

// #[derive(reflect::Input)]
// struct ParentStruct {
//     _f: GenericStruct<GenericStruct<u8>>,
// }

// #[derive(reflect::Input)]
// struct GenericStruct<A>
// where
//     A: reflect::Input,
// {
//     _f1: A,
// }

/// Some Enum docs
// /// more
// #[allow(unused_doc_comments, dead_code)]
// #[derive(reflect::Input)]
// enum MyEnum<
//     /// some generic param docs
//     /// multiline
//     T,
// > where
//     T: reflect::Input,
// {
//     /// Variant1 docs
//     Variant1(
//         /// variant1 field docs
//         T,
//     ),
//     /// Variant2 docs
//     /// multiline
//     /// more
//     /// more
//     Variant2 {
//         /// named field variant2 field docs
//         named_field: T,
//     },
// }

/// Some Struct docs
/// more
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflect::Input)]
struct TestStructDocumented {
    /// field docs
    /// multiline
    f: uuid::Uuid,
}

// #[derive(reflect::Input)]
// union MyUnion {
//     f: u8,
// }

// fn main() {
//     // println!("{:#?}", TypeAlias::reflect_input());
//     // //println!(
//     //     "{:#?}",
//     //     GenericStruct::<GenericStruct::<u8>>::reflect_input()
//     // );
//     println!("{:#?}", TestStructDocumented::reflect_input());

//     // //println!(
//     //     "{:#?}",
//     //     TestStructWithCircularReferenceGenericWithoutBox::<
//     //         TestStructWithCircularReferenceGenericWithoutBox::<u8>,
//     //     >::reflect_input()
//     // );
// }
