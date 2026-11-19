//! Calls each conformance route of the generated Rust client with serde's
//! samples, through a transport that records the request and serves the
//! sample back. `cargo test -p reflectapi-demo --test conformance` writes
//! the client, the samples and `cases.rs`.

use std::future::Future;
use std::sync::{Arc, Mutex};

use reflectapi_conformance_rust::Interface;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../target/conformance");

/// Records the request body and responds with the sample.
#[derive(Clone)]
struct Echo {
    base_url: reflectapi::rt::Url,
    response: bytes::Bytes,
    sent: Arc<Mutex<Option<(String, bytes::Bytes)>>>,
}

impl reflectapi::rt::Client for Echo {
    type Error = std::convert::Infallible;

    fn base_url(&self) -> &reflectapi::rt::Url {
        &self.base_url
    }

    async fn request(
        &self,
        request: reflectapi::rt::Request,
    ) -> Result<reflectapi::rt::Response<Self::Error>, Self::Error> {
        *self.sent.lock().unwrap() = Some((request.path, request.body));
        let body = self.response.clone();
        Ok(reflectapi::rt::Response {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Box::pin(futures::stream::once(async move { Ok(body) })),
        })
    }
}

struct Fixture {
    cases: serde_json::Map<String, Value>,
    divergences: Vec<Value>,
}

fn fixture() -> Fixture {
    let text = std::fs::read_to_string(format!("{ROOT}/samples.json"))
        .expect("run `cargo test -p reflectapi-demo --test conformance` first");
    let mut json: Value = serde_json::from_str(&text).unwrap();
    Fixture {
        cases: json["cases"].as_object_mut().unwrap().clone(),
        divergences: json["divergences"].as_array().unwrap().clone(),
    }
}

fn expected_failure(fixture: &Fixture, case: &str, sample: usize, check: &str) -> Option<String> {
    fixture.divergences.iter().find_map(|d| {
        (d["case"] == case
            && d["target"] == "rust"
            && d["check"] == check
            && (d["sample"].is_null() || d["sample"] == sample))
            .then(|| d["reason"].as_str().unwrap().to_owned())
    })
}

/// Sends each sample of `case` through `call`, and checks that the request
/// body and the re-encoded response are both the sample. A known divergence
/// must fail; one that passes is reported so its entry can be removed.
fn check<I, O, E, F, Fut>(case: &str, call: F)
where
    I: DeserializeOwned,
    O: Serialize,
    E: std::fmt::Debug,
    F: Fn(Interface<Echo>, I) -> Fut,
    Fut: Future<Output = Result<O, reflectapi::rt::Error<E, std::convert::Infallible>>>,
{
    let fixture = fixture();
    let entry = &fixture.cases[case];
    let mut failures = Vec::new();
    for (i, sample) in entry["samples"].as_array().unwrap().iter().enumerate() {
        let echo = Echo {
            base_url: "http://test".parse().unwrap(),
            response: serde_json::to_vec(sample).unwrap().into(),
            sent: Default::default(),
        };
        let sent = echo.sent.clone();
        let (request, response) = match serde_json::from_value::<I>(sample.clone()) {
            Err(e) => {
                let message = format!("the sample doesn't decode: {e}");
                (Some(message.clone()), Some(message))
            }
            Ok(input) => {
                let output = futures::executor::block_on(call(Interface::new(echo), input));
                let (path, body) = sent.lock().unwrap().take().expect("no request was sent");
                assert_eq!(path, entry["path"]);
                let request: Value = serde_json::from_slice(&body).unwrap();
                let response = match output {
                    Ok(output) => serde_json::to_value(output).unwrap(),
                    Err(error) => Value::String(format!("{error:?}")),
                };
                (
                    (&request != sample).then(|| format!("sent {request}")),
                    (&response != sample).then(|| format!("re-encoded as {response}")),
                )
            }
        };
        for (check, failure) in [("request", request), ("response", response)] {
            match (expected_failure(&fixture, case, i, check), failure) {
                (None, Some(message)) => {
                    failures.push(format!("sample {i} ({sample}), {check}: {message}"))
                }
                (Some(reason), None) => failures.push(format!(
                    "sample {i}, {check}: listed as a divergence ({reason}) but passes; \
                     remove it from DIVERGENCES"
                )),
                _ => {}
            }
        }
    }
    assert!(failures.is_empty(), "{case}:\n{}", failures.join("\n"));
}

include!("../../../../target/conformance/rust/cases.rs");
