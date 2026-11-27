//! The answer to a request over a call's rate limit: `429`, with a
//! `retry-after` header and a `rate_limited` case of the call's error type
//! that says, in seconds, how long to wait.
//!
//! ```http
//! 429 Too Many Requests
//! retry-after: 30
//! content-type: application/json
//!
//! {"kind": "rate_limited", "retry_after_seconds": 30}
//! ```
//!
//! [`RateLimited`] is a case of the call's error enum, marked
//! `#[reflectapi(transparent)]` so that the response takes its status and
//! header:
//!
//! ```
//! use reflectapi_conventions::rate_limited::RateLimited;
//!
//! #[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
//! #[serde(tag = "kind", rename_all = "snake_case")]
//! #[reflectapi(open)]
//! enum OrdersListError {
//!     /// Takes the status and the headers of `RateLimited`.
//!     #[reflectapi(transparent)]
//!     RateLimited(RateLimited),
//!     #[reflectapi(status = 403)]
//!     Forbidden,
//! }
//!
//! async fn orders_list(
//!     _: (),
//!     _: reflectapi::Empty,
//!     _: reflectapi::Empty,
//! ) -> Result<reflectapi::Empty, OrdersListError> {
//!     let over_limit = true;
//!     if over_limit {
//!         return Err(OrdersListError::RateLimited(RateLimited {
//!             retry_after_seconds: 30,
//!         }));
//!     }
//!     Ok(reflectapi::Empty {})
//! }
//!
//! let (schema, _) = reflectapi::Builder::<()>::new()
//!     .route(orders_list, |b| b.name("orders.list"))
//!     .build()
//!     .unwrap();
//! ```
//!
//! The header and the case carry the same number, as the header is made from
//! the case. A client's retry reads the header, which a `429` from a gateway
//! in front of the server has too; an app that shows "try again in 30
//! seconds" reads the case.

/// The request is over the call's rate limit: `429`, with `retry-after`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
    reflectapi::Output,
)]
pub struct RateLimited {
    /// Seconds to wait before retrying.
    pub retry_after_seconds: u64,
}

impl reflectapi::ResponseError for RateLimited {
    fn status(&self) -> http::StatusCode {
        http::StatusCode::TOO_MANY_REQUESTS
    }

    fn headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::RETRY_AFTER, self.retry_after_seconds.into());
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflectapi::ResponseError;

    #[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    #[reflectapi(open)]
    enum OrdersListError {
        #[reflectapi(transparent)]
        RateLimited(RateLimited),
        #[reflectapi(status = 403)]
        Forbidden,
    }

    #[test]
    fn a_rate_limited_case_answers_429_with_retry_after() {
        let error = OrdersListError::RateLimited(RateLimited {
            retry_after_seconds: 30,
        });
        assert_eq!(error.status(), http::StatusCode::TOO_MANY_REQUESTS);
        let headers = error.headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[http::header::RETRY_AFTER], "30");
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"kind":"rate_limited","retry_after_seconds":30}"#
        );

        let other = OrdersListError::Forbidden;
        assert_eq!(other.status(), http::StatusCode::FORBIDDEN);
        assert!(other.headers().is_empty());
    }
}
