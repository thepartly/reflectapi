// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: Demo application
// This is a demo application

#![allow(non_camel_case_types)]
#![allow(dead_code)]

pub use interface::Interface;
pub use reflectapi::rt::*;

pub mod interface {

    #[derive(Debug)]
    pub struct Interface<C: reflectapi::rt::Client + Clone> {
        pub health: HealthInterface<C>,
        pub pets: PetsInterface<C>,
        client: C,
        base_url: reflectapi::rt::Url,
    }

    impl<C: reflectapi::rt::Client + Clone> Interface<C> {
        pub fn try_new(
            client: C,
            base_url: reflectapi::rt::Url,
        ) -> std::result::Result<Self, reflectapi::rt::UrlParseError> {
            if base_url.cannot_be_a_base() {
                return Err(reflectapi::rt::UrlParseError::RelativeUrlWithCannotBeABaseBase);
            }

            Ok(Self {
                health: HealthInterface::try_new(client.clone(), base_url.clone())?,
                pets: PetsInterface::try_new(client.clone(), base_url.clone())?,
                client,
                base_url,
            })
        }
    }

    #[derive(Debug)]
    pub struct HealthInterface<C: reflectapi::rt::Client + Clone> {
        client: C,
        base_url: reflectapi::rt::Url,
    }

    impl<C: reflectapi::rt::Client + Clone> HealthInterface<C> {
        pub fn try_new(
            client: C,
            base_url: reflectapi::rt::Url,
        ) -> std::result::Result<Self, reflectapi::rt::UrlParseError> {
            if base_url.cannot_be_a_base() {
                return Err(reflectapi::rt::UrlParseError::RelativeUrlWithCannotBeABaseBase);
            }

            Ok(Self { client, base_url })
        }
        /// Check the health of the service
        #[tracing::instrument(name = "/health.check", skip(self, headers))]
        pub async fn check(
            &self,
            input: reflectapi::Empty,
            headers: reflectapi::Empty,
        ) -> Result<reflectapi::Empty, reflectapi::rt::Error<reflectapi::Empty, C::Error>> {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/health.check")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
    }

    #[derive(Debug)]
    pub struct PetsInterface<C: reflectapi::rt::Client + Clone> {
        client: C,
        base_url: reflectapi::rt::Url,
    }

    impl<C: reflectapi::rt::Client + Clone> PetsInterface<C> {
        pub fn try_new(
            client: C,
            base_url: reflectapi::rt::Url,
        ) -> std::result::Result<Self, reflectapi::rt::UrlParseError> {
            if base_url.cannot_be_a_base() {
                return Err(reflectapi::rt::UrlParseError::RelativeUrlWithCannotBeABaseBase);
            }

            Ok(Self { client, base_url })
        }
        /// List available pets
        #[tracing::instrument(name = "/pets.list", skip(self, headers))]
        pub async fn list(
            &self,
            input: super::types::myapi::proto::PetsListRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            super::types::myapi::proto::Paginated<super::types::myapi::model::output::Pet>,
            reflectapi::rt::Error<super::types::myapi::proto::PetsListError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.list")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
        /// Create a new pet
        #[tracing::instrument(name = "/pets.create", skip(self, headers))]
        pub async fn create(
            &self,
            input: super::types::myapi::proto::PetsCreateRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            reflectapi::rt::Error<super::types::myapi::proto::PetsCreateError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.create")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
        /// Update an existing pet
        #[tracing::instrument(name = "/pets.update", skip(self, headers))]
        pub async fn update(
            &self,
            input: super::types::myapi::proto::PetsUpdateRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            reflectapi::rt::Error<super::types::myapi::proto::PetsUpdateError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.update")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
        /// Remove an existing pet
        #[tracing::instrument(name = "/pets.remove", skip(self, headers))]
        pub async fn remove(
            &self,
            input: super::types::myapi::proto::PetsRemoveRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            reflectapi::rt::Error<super::types::myapi::proto::PetsRemoveError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.remove")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
        #[deprecated(note = "Use pets.remove instead")]
        /// Remove an existing pet
        #[tracing::instrument(name = "/pets.delete", skip(self, headers))]
        pub async fn delete(
            &self,
            input: super::types::myapi::proto::PetsRemoveRequest,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            reflectapi::Empty,
            reflectapi::rt::Error<super::types::myapi::proto::PetsRemoveError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.delete")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
        /// Fetch first pet, if any exists
        #[tracing::instrument(name = "/pets.get-first", skip(self, headers))]
        pub async fn get_first(
            &self,
            input: reflectapi::Empty,
            headers: super::types::myapi::proto::Headers,
        ) -> Result<
            std::option::Option<super::types::myapi::model::output::Pet>,
            reflectapi::rt::Error<super::types::myapi::proto::UnauthorizedError, C::Error>,
        > {
            reflectapi::rt::__request_impl(
                &self.client,
                self.base_url
                    .join("/pets.get-first")
                    .expect("checked base_url already and path is valid"),
                input,
                headers,
            )
            .await
        }
    }
}
pub mod types {

    pub mod myapi {
        pub mod model {

            #[derive(Debug, serde::Deserialize, serde::Serialize)]
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
                    #[serde(
                        default = "Default::default",
                        skip_serializing_if = "std::string::String::is_empty"
                    )]
                    notes: std::string::String,
                },
            }

            #[derive(Debug, serde::Deserialize, serde::Serialize)]
            #[serde(tag = "type")]
            pub enum Kind {
                /// A dog
                #[serde(rename = "dog")]
                Dog {
                    /// breed of the dog
                    breed: std::string::String,
                },
                /// A cat
                #[serde(rename = "cat")]
                Cat {
                    /// lives left
                    lives: u8,
                },
            }
            pub mod input {

                #[derive(Debug, serde::Serialize)]
                pub struct Pet {
                    /// identity
                    pub name: std::string::String,
                    /// kind of pet
                    pub kind: super::super::super::myapi::model::Kind,
                    /// age of the pet
                    #[serde(
                        default = "Default::default",
                        skip_serializing_if = "std::option::Option::is_none"
                    )]
                    #[deprecated(note = "test deprecation")]
                    pub age: std::option::Option<u8>,
                    #[serde(default = "Default::default")]
                    pub updated_at: chrono::DateTime<chrono::Utc>,
                    /// behaviors of the pet
                    #[serde(
                        default = "Default::default",
                        skip_serializing_if = "std::vec::Vec::is_empty"
                    )]
                    pub behaviors: std::vec::Vec<super::super::super::myapi::model::Behavior>,
                }
            }
            pub mod output {

                #[derive(Debug, serde::Deserialize)]
                pub struct Pet {
                    /// identity
                    pub name: std::string::String,
                    /// kind of pet
                    pub kind: super::super::super::myapi::model::Kind,
                    /// age of the pet
                    #[serde(
                        default = "Default::default",
                        skip_serializing_if = "std::option::Option::is_none"
                    )]
                    #[deprecated(note = "test deprecation")]
                    pub age: std::option::Option<u8>,
                    pub updated_at: chrono::DateTime<chrono::Utc>,
                    /// behaviors of the pet
                    #[serde(
                        default = "Default::default",
                        skip_serializing_if = "std::vec::Vec::is_empty"
                    )]
                    pub behaviors: std::vec::Vec<super::super::super::myapi::model::Behavior>,
                }
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
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "std::option::Option::is_none"
                )]
                pub cursor: std::option::Option<std::string::String>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsCreateError {
                Conflict,
                NotAuthorized,
                InvalidIdentity { message: std::string::String },
            }

            pub type PetsCreateRequest = super::super::myapi::model::input::Pet;

            #[derive(Debug, serde::Deserialize)]
            pub enum PetsListError {
                InvalidCursor,
                Unauthorized,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct PetsListRequest {
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "std::option::Option::is_none"
                )]
                pub limit: std::option::Option<u8>,
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "std::option::Option::is_none"
                )]
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
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "std::option::Option::is_none"
                )]
                pub kind: std::option::Option<super::super::myapi::model::Kind>,
                /// age of the pet, nullable in the model
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "reflectapi::Option::is_undefined"
                )]
                pub age: reflectapi::Option<u8>,
                /// behaviors of the pet, nullable in the model
                #[serde(
                    default = "Default::default",
                    skip_serializing_if = "reflectapi::Option::is_undefined"
                )]
                pub behaviors:
                    reflectapi::Option<std::vec::Vec<super::super::myapi::model::Behavior>>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct UnauthorizedError;
        }
    }
}
