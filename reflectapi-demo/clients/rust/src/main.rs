use reflectapi_demo_client_generated::{DemoServerClient, Error, ProtocolErrorStage};

#[tokio::main]
async fn main() {
    let client = DemoServerClient::try_new(
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
                println!("Network error: {:?}", e)
            }
            Error::Protocol { info, stage } => match stage {
                ProtocolErrorStage::SerializeRequestBody => {
                    eprint!("Failed to serialize request body: {}", info)
                }
                ProtocolErrorStage::SerializeRequestHeaders => {
                    eprint!("Failed to serialize request headers: {}", info)
                }
                ProtocolErrorStage::DeserializeResponseBody(body) => {
                    eprint!("Failed to deserialize response body: {}: {:#?}", info, body)
                }
                ProtocolErrorStage::DeserializeResponseError(status, body) => {
                    eprint!(
                        "Failed to deserialize response error: {} at {:?}: {:#?}",
                        info, status, body
                    )
                }
            },
            Error::Server(status, body) => {
                println!("Server error: {} with body: {:?}", status, body)
            }
        },
    }

    println!("done")
    // println!("{:#?}", result);
}
