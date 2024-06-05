// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: Demo application
// This is a demo application

use std::collections::HashMap;

pub trait Client<E> {
    async fn request(
        &self,
        path: &str,
        body: bytes::Bytes,
        headers: HashMap<String, String>,
    ) -> Result<(http::StatusCode, bytes::Bytes), E>;
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

async fn __request_impl<C, NE, I, H, O, E>(
    client: &C,
    path: &str,
    body: I,
    headers: H,
) -> Result<O, Error<E, NE>>
where
    C: Client<NE>,
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

    let mut headers_serialized = HashMap::new();
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
        _ => {
            return Err(Error::Protocol {
                info: "Headers must be an object".to_string(),
                stage: ProtocolErrorStage::SerializeRequestHeaders,
            });
        }
    }
    let (status, body) = client
        .request(path, body, headers_serialized)
        .await
        .map_err(Error::Network)?;
    if status.is_success() {
        let output = serde_json::from_slice(&body).map_err(|e| Error::Protocol {
            info: e.to_string(),
            stage: ProtocolErrorStage::DeserializeResponseBody(body),
        })?;
        Ok(output)
    } else if status.is_client_error() {
        match serde_json::from_slice::<E>(&body) {
            Ok(error) => Err(Error::Application(error)),
            Err(e) => Err(Error::Protocol {
                info: e.to_string(),
                stage: ProtocolErrorStage::DeserializeResponseError(status, body),
            }),
        }
    } else {
        Err(Error::Server(status, body))
    }
}


#[cfg(feature = "reqwest")]
pub struct ReqwestClient {
    client: reqwest::Client,
    base_url: String,
}

#[cfg(feature = "reqwest")]
impl ReqwestClient {
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[cfg(feature = "reqwest")]
impl Client<reqwest::Error> for ReqwestClient {
    async fn request(
        &self,
        path: &str,
        body: bytes::Bytes,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<(http::StatusCode, bytes::Bytes), reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url);
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

pub mod interface {
    use std::marker::PhantomData;

    use super::{Client, __request_impl};

    pub struct Interface<E, C: Client<E>> {
        pub health: Health<E, C>,
        // pets: Pets<E, C>,
    }

    impl<E, C: Client<E>> Interface<E, C> {
        pub fn new(client: C) -> Self {
            Self {
                health: Health::new(client),
                // pets: Pets::new(client),
            }
        }
    }

    pub struct Health<E, C>
    where
        C: Client<E>,
    {
        client: C,
        _marker: PhantomData<E>,
    }

    impl<E, C: Client<E>> Health<E, C> {
        pub fn new(client: C) -> Self {
            Self {
                client,
                _marker: PhantomData,
            }
        }

        pub async fn check(&self, input: (), headers: ()) -> Result<(), super::Error<(), E>> {
            __request_impl(&self.client, "health.check", input, headers).await
        }
    }

    // struct Pets<E, C: Client<E>> {
    //     client: C,
    // }
}

pub async fn demo() {
    // making a request client and initializing typed client interface once per app
    // note we use generated default provided implementation of the Client trait by Reqwest...
    let client = ReqwestClient::new(reqwest::Client::new(), "http://localhost:8080".to_string());
    // .. it is possible to use other implementations of a Client trait
    let interface = interface::Interface::new(client);

    // making a call
    let result = interface.health.check((), ()).await;

    // similarly code will look for:
    // let result = interface.sellables.ingest(IngestRequest { sellables: ..., gapc_parts: ... }, IngestHeaders { authorization: ... }).await;

    // error handling demo:
    match result {
        Ok(v) => {
            // use structured application response data here
            println!("Health check successful")
        }
        Err(e) => match e {
            Error::Application(v) => {
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
                    eprint!("Failed to deserialize response body: {}", info)
                }
                ProtocolErrorStage::DeserializeResponseError(status, body) => {
                    eprint!(
                        "Failed to deserialize response error: {} at {:?}",
                        info, status
                    )
                }
            },
            Error::Server(status, body) => {
                println!("Server error: {} with body {:?}", status, body)
            }
        },
    }
}

// pub fn client<E, C: Client<E>>(client: C) -> __definition::Interface {
//     return __implementation.__client(base);
// }

// pub mod __definition {
//     pub mod Interface {}
// }

// export namespace __definition {

// export interface Interface {
//     health: health.Interface,
//     pets: pets.Interface,
// }

// export namespace health {

// export interface Interface {
//     /// Check the health of the service
//     check: (input: {}, headers: {})
//         => AsyncResult<{}, {}>,
// }

// }

// export namespace pets {

// export interface Interface {
//     /// List available pets
//     list: (input: myapi.proto.PetsListRequest, headers: myapi.proto.Headers)
//         => AsyncResult<myapi.proto.Paginated<myapi.model.Pet>, myapi.proto.PetsListError>,
//     /// Create a new pet
//     create: (input: myapi.proto.PetsCreateRequest, headers: myapi.proto.Headers)
//         => AsyncResult<{}, myapi.proto.PetsCreateError>,
//     /// Update an existing pet
//     update: (input: myapi.proto.PetsUpdateRequest, headers: myapi.proto.Headers)
//         => AsyncResult<{}, myapi.proto.PetsUpdateError>,
//     /// Remove an existing pet
//     remove: (input: myapi.proto.PetsRemoveRequest, headers: myapi.proto.Headers)
//         => AsyncResult<{}, myapi.proto.PetsRemoveError>,
//     /// Fetch first pet, if any exists
//     get_first: (input: {}, headers: myapi.proto.Headers)
//         => AsyncResult<myapi.model.Pet | null, myapi.proto.UnauthorizedError>,
// }

// }

// }

// export namespace myapi {

// export namespace model {

// export type Behavior =
//     | "Calm"
//     | {
//         Aggressive: [
//             /// aggressiveness level
//             number /* f64 */,
//             /// some notes
//             string
//         ]
//     }
//     | {
//         Other: {
//             /// Custom provided description of a behavior
//             description: string,
//             /// Additional notes
//             /// Up to a user to put free text here
//             notes?: string
//         }
//     };

// export type Kind =
//     /// A dog
//     | "dog"
//     /// A cat
//     | "cat";

// export interface Pet {
//     /// identity
//     name: string,
//     /// kind of pet
//     kind: myapi.model.Kind,
//     /// age of the pet
//     age?: number /* u8 */ | null,
//     /// behaviors of the pet
//     behaviors?: Array<myapi.model.Behavior>,
// }

// }

// export namespace proto {

// export interface Headers {
//     authorization: string,
// }

// export interface Paginated<T> {
//     /// slice of a collection
//     items: Array<T>,
//     /// cursor for getting next page
//     cursor?: string | null,
// }

// export type PetsCreateError =
//     | "Conflict"
//     | "NotAuthorized"
//     | {
//         InvalidIdentity: {
//             message: string
//         }
//     };

// export type PetsCreateRequest = myapi.model.Pet;

// export type PetsListError =
//     | "InvalidCustor"
//     | "Unauthorized";

// export interface PetsListRequest {
//     limit?: number /* u8 */ | null,
//     cursor?: string | null,
// }

// export type PetsRemoveError =
//     | "NotFound"
//     | "NotAuthorized";

// export interface PetsRemoveRequest {
//     /// identity
//     name: string,
// }

// export type PetsUpdateError =
//     | "NotFound"
//     | "NotAuthorized";

// export interface PetsUpdateRequest {
//     /// identity
//     name: string,
//     /// kind of pet, non nullable in the model
//     kind?: myapi.model.Kind | null,
//     /// age of the pet, nullable in the model
//     age?: number /* u8 */ | null | undefined,
//     /// behaviors of the pet, nullable in the model
//     behaviors?: Array<myapi.model.Behavior> | null | undefined,
// }

// export type UnauthorizedError = null;

// }

// }

// export namespace reflect {

// /// Struct object with no fields
// export interface Empty {
// }

// /// Error object which is expected to be never returned
// export interface Infallible {
// }

// }

// namespace __implementation {

// export function __client(base: string | Client): __definition.Interface {
//     const client_instance = typeof base === 'string' ? new ClientInstance(base) : base;
//     return { impl: {
//         health: {
//             check: health__check(client_instance),
//         },
//         pets: {
//             list: pets__list(client_instance),
//             create: pets__create(client_instance),
//             update: pets__update(client_instance),
//             remove: pets__remove(client_instance),
//             get_first: pets__get_first(client_instance),
//         },
//     }, }.impl
// }

// export function __request<I, H, O, E>(client: Client, path: string, input: I | undefined, headers: H | undefined): AsyncResult<O, E> {
//     let hdrs: Record<string, string> = {
//         'content-type': 'application/json',
//     };
//     if (headers) {
//         for (const [k, v] of Object.entries(headers)) {
//             hdrs[k?.toString()] = v?.toString() || '';
//         }
//     }
//     return client.request(path, JSON.stringify(input), hdrs)
//         .then(([status, response_body]) => {
//             if (status < 200 || status >= 300) {
//                 let parsed_response_body;
//                 try {
//                     parsed_response_body = JSON.parse(response_body)
//                 } catch (e) {
//                     return new Result<O, Err<E>>({ err: new Err({ other_err: response_body }) });
//                 }
//                 return new Result<O, Err<E>>({ err: new Err({ application_err: parsed_response_body as E }) });
//             }

//             let parsed_response_body;
//             try {
//                  parsed_response_body = JSON.parse(response_body)
//             } catch (e) {
//                 return new Result<O, Err<E>>({
//                     err: new Err({
//                         other_err:
//                             'internal error: failure to parse response body as json on successful status code: ' + response_body
//                     })
//                 });
//             }
//             return new Result<O, Err<E>>({ ok: parsed_response_body as O });
//         }
//         ).catch((e) => {
//             return new Result<O, Err<E>>({ err: new Err({ other_err: e }) });
//         });
// }

// class ClientInstance {
//     constructor(private base: string) {}

//     public request(path: string, body: string, headers: Record<string, string>): Promise<[number, string]> {
//         return fetch(`${this.base}/${path}`, {
//             method: 'POST',
//             headers: headers,
//             body: body,
//         }).then((response) => {
//             return response.text().then((text) => {
//                 return [response.status, text];
//             });
//         });
//     }
// }

// function health__check(client: Client) {
//     return (input: {}, headers: {}) => __request<
//         {}, {}, {}, {}
//     >(client, 'health.check', input, headers);
// }
// function pets__list(client: Client) {
//     return (input: myapi.proto.PetsListRequest, headers: myapi.proto.Headers) => __request<
//         myapi.proto.PetsListRequest, myapi.proto.Headers, myapi.proto.Paginated<myapi.model.Pet>, myapi.proto.PetsListError
//     >(client, 'pets.list', input, headers);
// }
// function pets__create(client: Client) {
//     return (input: myapi.proto.PetsCreateRequest, headers: myapi.proto.Headers) => __request<
//         myapi.proto.PetsCreateRequest, myapi.proto.Headers, {}, myapi.proto.PetsCreateError
//     >(client, 'pets.create', input, headers);
// }
// function pets__update(client: Client) {
//     return (input: myapi.proto.PetsUpdateRequest, headers: myapi.proto.Headers) => __request<
//         myapi.proto.PetsUpdateRequest, myapi.proto.Headers, {}, myapi.proto.PetsUpdateError
//     >(client, 'pets.update', input, headers);
// }
// // function pets__remove(client: Client) {
//     return (input: myapi.proto.PetsRemoveRequest, headers: myapi.proto.Headers) => __request<
//         myapi.proto.PetsRemoveRequest, myapi.proto.Headers, {}, myapi.proto.PetsRemoveError
//     >(client, 'pets.remove', input, headers);
// }
// function pets__get_first(client: Client) {
//     return (input: {}, headers: myapi.proto.Headers) => __request<
//         {}, myapi.proto.Headers, myapi.model.Pet | null, myapi.proto.UnauthorizedError
//     >(client, 'pets.get-first', input, headers);
// }

// }
