use futures_util::StreamExt;
use reflectapi::rt::{Error, ProtocolErrorStage};
use reflectapi_demo_client_generated::types::myapi::model::input::Pet;
use reflectapi_demo_client_generated::types::myapi::model::Kind;
use reflectapi_demo_client_generated::types::myapi::proto::Headers;
use reflectapi_demo_client_generated::DemoServerClient;

type Client = DemoServerClient<reflectapi::rt::ReqwestClient>;

fn headers() -> Headers {
    Headers {
        authorization: "password".into(),
    }
}

fn pet(name: &str, kind: Kind) -> Pet {
    #[allow(deprecated)]
    Pet {
        name: name.into(),
        kind,
        age: None,
        updated_at: Default::default(),
        behaviors: vec![],
    }
}

#[tokio::main]
async fn main() {
    let client: Client = DemoServerClient::try_new(
        reqwest::Client::new(),
        "http://localhost:3000".parse().unwrap(),
    )
    .unwrap();

    let result = client
        .health
        .check(reflectapi::Empty {}, reflectapi::Empty {})
        .await;
    // error handling demo:
    match result {
        Ok(_v) => {
            // use structured application response data here
            println!("Health check successful")
        }
        Err(e) => match e {
            Error::Application(_v) => {
                // use structured application error here
                println!("Health check failed")
            }
            Error::Network(e) => {
                println!("Network error: {e:?}")
            }
            Error::Protocol { info, stage } => match stage {
                ProtocolErrorStage::SerializeRequestBody => {
                    eprint!("Failed to serialize request body: {info}")
                }
                ProtocolErrorStage::SerializeRequestHeaders => {
                    eprint!("Failed to serialize request headers: {info}")
                }
                ProtocolErrorStage::DeserializeResponseBody(body) => {
                    eprint!("Failed to deserialize response body: {info}: {body:#?}")
                }
                ProtocolErrorStage::DeserializeResponseError(status, body) => {
                    eprint!("Failed to deserialize response error: {info} at {status:?}: {body:#?}")
                }
            },
            Error::Server(status, body) => {
                println!("Server error: {status} with body: {body:?}")
            }
        },
    }

    println!("streaming cdc events while mutating pets");
    let mut stream = client
        .pets
        .cdc_events(reflectapi::Empty {}, headers())
        .await
        .expect("start stream");

    let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let received_clone = received.clone();

    let stream_handle = tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match item {
                Ok(p) => {
                    println!("received event: {} {:?}", p.name, p.kind);
                    received_clone.lock().await.push(p.name);
                }
                Err(e) => {
                    eprintln!("stream error: {e:?}");
                    break;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("creating Whiskers");
    client
        .pets
        .create(pet("Whiskers", Kind::Cat { lives: 9 }), headers())
        .await
        .expect("create Whiskers");

    println!("creating Tweety");
    client
        .pets
        .create(pet("Tweety", Kind::Bird), headers())
        .await
        .expect("create Tweety");

    println!("removing Whiskers");
    client
        .pets
        .remove(
            reflectapi_demo_client_generated::types::myapi::proto::PetsRemoveRequest {
                name: "Whiskers".into(),
            },
            headers(),
        )
        .await
        .expect("remove Whiskers");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    stream_handle.abort();
    let _ = stream_handle.await;

    let received = received.lock().await;
    let expected = vec!["Whiskers", "Tweety", "Whiskers"];
    if *received == expected {
        println!("stream test passed");
    } else {
        println!(
            "stream test FAILED: expected {:?}, got {:?}",
            expected, *received
        );
    }

    println!("removing remaining pets");
    let remove = |name: &'static str| {
        let client = &client;
        async move {
            let _ = client
                .pets
                .remove(
                    reflectapi_demo_client_generated::types::myapi::proto::PetsRemoveRequest {
                        name: name.into(),
                    },
                    headers(),
                )
                .await
                .inspect_err(|err| eprintln!("failed to remove {name}: {err:?}"));
        }
    };

    remove("Tweety").await;
    remove("BadPet").await;
    remove("GoodPet").await;

    println!("done");
}
