use futures_util::StreamExt;
use reflectapi_demo_client_generated::types::myapi::model::input::Pet;
use reflectapi_demo_client_generated::types::myapi::model::Kind;
use reflectapi_demo_client_generated::types::myapi::proto::Headers;
use reflectapi_demo_client_generated::DemoServerClient;

type Client = DemoServerClient<reqwest::Client>;

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

    println!("streaming cdc events while mutating pets");
    let mut stream = client
        .pets
        .cdc_events(reflectapi::Empty {}, headers())
        .await
        .expect("start stream");

    let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let received_clone = received.clone();

    let stream_handle = tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match item {
                Ok(p) => {
                    println!("received event: {} {:?}", p.name, p.kind);
                    received_clone.lock().unwrap().push(p.name);
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

    let received = received.lock().unwrap();
    let expected = vec!["Whiskers", "Tweety", "Whiskers"];
    if *received == expected {
        println!("stream test passed");
    } else {
        println!(
            "stream test FAILED: expected {:?}, got {:?}",
            expected, *received
        );
    }

    // Test fallible stream
    println!("streaming fallible cdc events while mutating pets");
    let mut fallible_stream = client
        .pets
        .cdc_events_fallible(reflectapi::Empty {}, headers())
        .await
        .expect("start fallible stream");

    let received_fallible = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let received_fallible_clone = received_fallible.clone();

    let fallible_handle = tokio::spawn(async move {
        while let Some(item) = fallible_stream.next().await {
            match item {
                Ok(Ok(p)) => {
                    println!("fallible received ok: {} {:?}", p.name, p.kind);
                    received_fallible_clone
                        .lock()
                        .unwrap()
                        .push(format!("ok:{}", p.name));
                }
                Ok(Err(e)) => {
                    println!("fallible received err: {}", e.message);
                    received_fallible_clone
                        .lock()
                        .unwrap()
                        .push(format!("err:{}", e.message));
                }
                Err(e) => {
                    eprintln!("fallible stream error: {e:?}");
                    break;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("creating BadPet");
    client
        .pets
        .create(pet("BadPet", Kind::Bird), headers())
        .await
        .expect("create BadPet");

    println!("creating GoodPet");
    client
        .pets
        .create(pet("GoodPet", Kind::Cat { lives: 7 }), headers())
        .await
        .expect("create GoodPet");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    fallible_handle.abort();
    let _ = fallible_handle.await;

    let received_fallible = received_fallible.lock().unwrap();
    let expected_fallible = vec!["err:Something went wrong with this pet", "ok:GoodPet"];
    if *received_fallible == expected_fallible {
        println!("fallible stream test passed");
    } else {
        println!(
            "fallible stream test FAILED: expected {:?}, got {:?}",
            expected_fallible, *received_fallible
        );
    }

    println!("removing remaining pets");
    let remove = |name: &'static str| {
        let client = &client;
        async move {
            client
                .pets
                .remove(
                    reflectapi_demo_client_generated::types::myapi::proto::PetsRemoveRequest {
                        name: name.into(),
                    },
                    headers(),
                )
                .await
                .expect(&format!("remove {name}"));
        }
    };
    remove("Tweety").await;
    remove("BadPet").await;
    remove("GoodPet").await;

    println!("done");
}
