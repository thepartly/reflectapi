//! Request headers types with number and bool fields, read through a route.

use reflectapi::dispatch::{Dispatcher, Reply};
use reflectapi::{Empty, Input, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(serde::Deserialize, Input)]
struct PagingHeaders {
    #[serde(rename = "x-limit")]
    limit: u32,
    #[serde(rename = "x-dry-run")]
    dry_run: Option<bool>,
}

#[derive(serde::Serialize, Output)]
struct Paging {
    limit: u32,
    dry_run: Option<bool>,
}

async fn paging(_: (), _: Empty, headers: PagingHeaders) -> Paging {
    Paging {
        limit: headers.limit,
        dry_run: headers.dry_run,
    }
}

fn dispatcher() -> Result<Dispatcher<()>, reflectapi::BuildErrors> {
    let (_schema, routers) = reflectapi::Builder::<()>::new()
        .route(paging, |b| b.name("paging"))
        .build()?;
    Ok(Dispatcher::new(routers))
}

/// The route's status and body for these headers.
async fn call(
    headers: &[(&'static str, &'static str)],
) -> Result<(u16, String), Box<dyn std::error::Error>> {
    let mut map = http::HeaderMap::new();
    map.insert(
        "content-type",
        http::HeaderValue::from_static("application/json"),
    );
    for (name, value) in headers {
        map.append(*name, http::HeaderValue::from_static(value));
    }
    let reply = dispatcher()?
        .dispatch(
            (),
            &http::Method::POST,
            "/paging",
            &map,
            bytes::Bytes::new(),
        )
        .await
        .ok_or("no route")?;
    let Reply::Complete { status, body, .. } = reply else {
        return Err("expected a complete reply".into());
    };
    Ok((status.as_u16(), String::from_utf8(body.to_vec())?))
}

#[tokio::test]
async fn number_and_bool_headers_are_parsed() -> TestResult {
    assert_eq!(
        call(&[("x-limit", "25"), ("x-dry-run", "true")]).await?,
        (200, r#"{"limit":25,"dry_run":true}"#.into())
    );
    assert_eq!(
        call(&[("x-limit", "25")]).await?,
        (200, r#"{"limit":25,"dry_run":null}"#.into())
    );
    Ok(())
}

#[tokio::test]
async fn a_header_that_doesnt_parse_is_a_400_saying_why() -> TestResult {
    let (status, body) = call(&[("x-limit", "lots")]).await?;
    assert_eq!(status, 400);
    assert_eq!(
        body,
        "Failed to parse request headers: invalid value: string \"lots\", expected u32"
    );
    Ok(())
}

#[tokio::test]
async fn a_repeated_header_is_read_as_its_first_value() -> TestResult {
    assert_eq!(
        call(&[("x-limit", "1"), ("x-limit", "2")]).await?,
        (200, r#"{"limit":1,"dry_run":null}"#.into())
    );
    Ok(())
}
