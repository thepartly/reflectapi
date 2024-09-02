// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: Demo application
// This is a demo application

#![allow(non_camel_case_types)]
#![allow(dead_code)]

pub use interface::Interface;

pub mod interface {

    #[derive(Debug)]
    pub struct Interface<C: super::Client + Clone> {
        pub health: HealthInterface<C>,
        pub pets: PetsInterface<C>,
        client: C,
        base_url: std::string::String,
    }

    impl<C: super::Client + Clone> Interface<C> {
        pub fn new(client: C, base_url: std::string::String) -> Self {
            Self {
                health: HealthInterface::new(client.clone(), base_url.clone()),
                pets: PetsInterface::new(client.clone(), base_url.clone()),
                client,
                base_url,
            }
        }
    }

    #[derive(Debug)]
    pub struct HealthInterface<C: super::Client + Clone> {
        client: C,
        base_url: std::string::String,
    }

    impl<C: super::Client + Clone> HealthInterface<C> {
        pub fn new(client: C, base_url: std::string::String) -> Self {
            Self { client, base_url }
        }
        /// Check the health of the service
        pub async fn check(
            &self,
            input: reflectapi::Empty,
            headers: reflectapi::Empty,
        ) -> Result<reflectapi::Empty, super::Error<reflectapi::Empty, C::Error>> {
            super::__request_impl(
                &self.client,
                &self.base_url,
                "/health.check",
                input,
                headers,
            )
            .await
        }
    }

    #[derive(Debug)]
    pub struct PetsInterface<C: super::Client + Clone> {
        client: C,
        base_url: std::string::String,
    }

    impl<C: super::Client + Clone> PetsInterface<C> {
        pub fn new(client: C, base_url: std::string::String) -> Self {
            Self { client, base_url }
        }
        /// List available pets
        pub async fn list(
            &self,
            input: super::types::myapi::proto::PetsListRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            super::types::myapi::proto::Paginated<super::types::myapi::model::Pet>,
            super::Error<super::types::myapi::proto::PetsListError, C::Error>,
        > {
            super::__request_impl(&self.client, &self.base_url, "/pets.list", input, headers).await
        }
        /// Create a new pet
        pub async fn create(
            &self,
            input: super::types::myapi::proto::PetsCreateRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            super::Error<super::types::myapi::proto::PetsCreateError, C::Error>,
        > {
            super::__request_impl(&self.client, &self.base_url, "/pets.create", input, headers)
                .await
        }
        /// Update an existing pet
        pub async fn update(
            &self,
            input: super::types::myapi::proto::PetsUpdateRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            super::Error<super::types::myapi::proto::PetsUpdateError, C::Error>,
        > {
            super::__request_impl(&self.client, &self.base_url, "/pets.update", input, headers)
                .await
        }
        /// Remove an existing pet
        pub async fn remove(
            &self,
            input: super::types::myapi::proto::PetsRemoveRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            super::Error<super::types::myapi::proto::PetsRemoveError, C::Error>,
        > {
            super::__request_impl(&self.client, &self.base_url, "/pets.remove", input, headers)
                .await
        }
        /// Fetch first pet, if any exists
        pub async fn get_first(
            &self,
            input: reflectapi::Empty,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            std::option::Option<super::types::myapi::model::Pet>,
            super::Error<super::types::myapi::proto::UnauthorizedError, C::Error>,
        > {
            super::__request_impl(
                &self.client,
                &self.base_url,
                "/pets.get-first",
                input,
                headers,
            )
            .await
        }
    }
}

pub trait Client {
    type Error;

    fn request(
        &self,
        path: &str,
        body: bytes::Bytes,
        headers: std::collections::HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<(http::StatusCode, bytes::Bytes), Self::Error>>;
}

pub enum Error<AE, NE> {
    Application(AE),
    Network(NE),
    Protocol {
        info: String,
        stage: ProtocolErrorStage,
    },
    Server(http::StatusCode, bytes::Bytes),
}

pub enum ProtocolErrorStage {
    SerializeRequestBody,
    SerializeRequestHeaders,
    DeserializeResponseBody(bytes::Bytes),
    DeserializeResponseError(http::StatusCode, bytes::Bytes),
}

#[cfg(feature = "reqwest")]
impl Client for reqwest::Client {
    type Error = reqwest::Error;

    async fn request(
        &self,
        path: &str,
        body: bytes::Bytes,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<(http::StatusCode, bytes::Bytes), Self::Error> {
        let mut request = self.post(path);
        for (k, v) in headers {
            request = request.header(k, v);
        }
        let response = request.body(body).send().await;
        let response = match response {
            Ok(response) => response,
            Err(e) => return Err(e),
        };
        let status = response.status();
        let body = response.bytes().await;
        let body = match body {
            Ok(body) => body,
            Err(e) => return Err(e),
        };
        Ok((status, body))
    }
}

pub mod types {
    pub mod myapi {
        pub mod model {

            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub enum Behavior {
                Calm,
                Aggressive(
                    /// aggressiveness level
                    f64,
                    /// some notes
                    std::string::String,
                ),
                Other {
                    /// Custom provided description of a behavior
                    description: std::string::String,
                    /// Additional notes
                    /// Up to a user to put free text here
                    #[serde(default, skip_serializing_if = "std::string::String::is_empty")]
                    notes: std::string::String,
                },
            }

            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub enum Kind {
                /// A dog
                #[serde(rename = "dog")]
                Dog,
                /// A cat
                #[serde(rename = "cat")]
                Cat,
            }

            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub struct Pet {
                /// identity
                pub name: std::string::String,
                /// kind of pet
                pub kind: super::super::myapi::model::Kind,
                /// age of the pet
                #[serde(default, skip_serializing_if = "std::option::Option::is_none")]
                pub age: std::option::Option<u8>,
                /// behaviors of the pet
                #[serde(default, skip_serializing_if = "std::vec::Vec::is_empty")]
                pub behaviors: std::vec::Vec<super::super::myapi::model::Behavior>,
            }
        }
        pub mod proto {

            #[derive(Debug, serde::Serialize)]
            pub struct Headers {
                pub authorization: std::string::String,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct Paginated<T> {
                /// slice of a collection
                pub items: std::vec::Vec<T>,
                /// cursor for getting next page
                #[serde(default, skip_serializing_if = "std::option::Option::is_none")]
                pub cursor: std::option::Option<std::string::String>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsCreateError {
                Conflict,
                NotAuthorized,
                InvalidIdentity { message: std::string::String },
            }

            pub type PetsCreateRequest = super::super::myapi::model::Pet;

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsListError {
                InvalidCustor,
                Unauthorized,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct PetsListRequest {
                #[serde(default, skip_serializing_if = "std::option::Option::is_none")]
                pub limit: std::option::Option<u8>,
                #[serde(default, skip_serializing_if = "std::option::Option::is_none")]
                pub cursor: std::option::Option<std::string::String>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsRemoveError {
                NotFound,
                NotAuthorized,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct PetsRemoveRequest {
                /// identity
                pub name: std::string::String,
            }

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsUpdateError {
                NotFound,
                NotAuthorized,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct PetsUpdateRequest {
                /// identity
                pub name: std::string::String,
                /// kind of pet, non nullable in the model
                #[serde(default, skip_serializing_if = "std::option::Option::is_none")]
                pub kind: std::option::Option<super::super::myapi::model::Kind>,
                /// age of the pet, nullable in the model
                #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
                pub age: reflectapi::Option<u8>,
                /// behaviors of the pet, nullable in the model
                #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
                pub behaviors:
                    reflectapi::Option<std::vec::Vec<super::super::myapi::model::Behavior>>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct UnauthorizedError;
        }
    }
}

async fn __request_impl<C, I, H, O, E>(
    client: &C,
    base_url: &str,
    path: &str,
    body: I,
    headers: H,
) -> Result<O, Error<E, C::Error>>
where
    C: Client,
    I: serde::Serialize,
    H: serde::Serialize,
    O: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned,
{
    let body = serde_json::to_vec(&body).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestBody,
    })?;
    let body = bytes::Bytes::from(body);
    let headers = serde_json::to_value(&headers).map_err(|e| Error::Protocol {
        info: e.to_string(),
        stage: ProtocolErrorStage::SerializeRequestHeaders,
    })?;

    let mut headers_serialized = std::collections::HashMap::new();
    match headers {
        serde_json::Value::Object(headers) => {
            for (k, v) in headers.into_iter() {
                let v_str = match v {
                    serde_json::Value::String(v) => v,
                    v => v.to_string(),
                };
                headers_serialized.insert(k, v_str);
            }
        }
        serde_json::Value::Null => {}
        _ => {
            return Err(Error::Protocol {
                info: "Headers must be an object".to_string(),
                stage: ProtocolErrorStage::SerializeRequestHeaders,
            });
        }
    }
    let (status, body) = client
        .request(&format!("{}{}", base_url, path), body, headers_serialized)
        .await
        .map_err(Error::Network)?;
    if status.is_success() {
        let output = serde_json::from_slice(&body).map_err(|e| Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseBody(body),
        })?;
        return Ok(output);
    }
    match serde_json::from_slice::<E>(&body) {
        Ok(error) => Err(Error::Application(error)),
        Err(e) if status.is_client_error() => Err(Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseError(status, body),
        }),
        Err(_) => Err(Error::Server(status, body)),
    }
}
