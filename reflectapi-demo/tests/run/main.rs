use core::fmt;

use anyhow::anyhow;
use axum::body::Body;
use http_rest_file::model::WithDefault;
use tower::Service as _;

#[tokio::main(flavor = "current_thread")]
async fn run(path: &datatest_stable::Utf8Path) -> datatest_stable::Result<()> {
    let file = http_rest_file::Parser::parse_file(path.as_std_path())?;

    let reqs = rest_file_to_reqs(file)?;

    let (_schema, router) = reflectapi_demo::builder().build().unwrap();
    let mut app = reflectapi::axum::into_router(Default::default(), router, |_name, r| r);

    let mut pretty = String::new();
    for req in reqs {
        let response = app.call(req).await?;

        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, 10000).await?;

        let response = ResponsePretty {
            parts,
            body: String::from_utf8_lossy(&body).to_string(),
        };

        pretty.push_str(&format!("{response}\n\n"));
    }

    insta::assert_snapshot!(
        path.file_name()
            .unwrap()
            .replace('/', "__")
            .strip_suffix(".http"),
        pretty
    );

    Ok(())
}

struct ResponsePretty {
    parts: http::response::Parts,
    body: String,
}

impl fmt::Display for ResponsePretty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?} {}", self.parts.version, self.parts.status)?;
        writeln!(f)?;

        let mut sorted_headers = self.parts.headers.iter().collect::<Vec<_>>();
        sorted_headers.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));

        for (key, value) in sorted_headers {
            writeln!(f, "{key}: {}", value.to_str().unwrap_or_default())?;
        }

        writeln!(f)?;
        writeln!(f, "{}", self.body)
    }
}

fn rest_file_to_reqs(
    file: http_rest_file::model::HttpRestFile,
) -> anyhow::Result<Vec<http::Request<Body>>> {
    if !file.errs.is_empty() {
        return Err(anyhow!("file has errors: {:?}", file.errs));
    }

    Ok(file.requests.into_iter().map(req_to_http_req).collect())
}

fn req_to_http_req(req: http_rest_file::model::Request) -> http::Request<Body> {
    use http_rest_file::model::{DataSource, RequestBody, RequestTarget};

    let method = match req.request_line.method {
        WithDefault::Some(method) => method,
        WithDefault::Default(method) => method,
    };
    let mut builder = http::Request::builder()
        .method(http::Method::try_from(method.to_string().as_str()).unwrap())
        .uri(match req.request_line.target {
            RequestTarget::RelativeOrigin { uri } | RequestTarget::Absolute { uri } => uri,
            RequestTarget::Asterisk | RequestTarget::InvalidTarget(_) | RequestTarget::Missing => {
                unimplemented!("unsupported request target")
            }
        });

    for header in req.headers {
        builder = builder.header(header.key, header.value);
    }

    builder
        .body(Body::from(match req.body {
            RequestBody::None => vec![],
            RequestBody::Raw { data } => match data {
                DataSource::Raw(s) => s.into_bytes(),
                DataSource::FromFilepath(_) => todo!(),
            },
            RequestBody::Multipart { .. } => todo!(),
            RequestBody::UrlEncoded { .. } => todo!(),
        }))
        .unwrap()
}

datatest_stable::harness!(run, "requests", r".*.http");
