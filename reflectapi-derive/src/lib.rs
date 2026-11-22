mod context;
mod derive;
mod parser;
mod status_code;
mod symbol;
mod tokenizable_schema;

/// The guide's page on the `#[reflectapi(...)]` attributes, named in errors.
const ATTRIBUTES_GUIDE: &str =
    "https://reflectapi.rs/getting-started/attributes.html";

/// The guide's section on a route's error, named in `StatusCode` errors.
const ROUTE_ERROR_GUIDE: &str =
    "https://reflectapi.rs/getting-started/attributes.html#a-routes-error";

/// Describes a type a route reads, a request body or headers, in the schema.
///
/// See [the attributes it reads](https://reflectapi.rs/getting-started/attributes.html).
#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Input, attributes(reflectapi, serde))]
pub fn derive_reflectapi_input(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Input)
}

/// Describes a type a route returns, a response body or error, in the schema.
///
/// See [the attributes it reads](https://reflectapi.rs/getting-started/attributes.html).
#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Output, attributes(reflectapi, serde))]
pub fn derive_reflectapi_output(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Output)
}

/// Implements `reflectapi::StatusCode` for an error enum. Each variant has its
/// status, `#[reflectapi(status = 404)]`, or takes the status and headers of
/// its one field, `#[reflectapi(transparent)]`.
///
/// See [A route's error](https://reflectapi.rs/getting-started/attributes.html#a-routes-error).
#[proc_macro_derive(StatusCode, attributes(reflectapi))]
pub fn derive_reflectapi_status_code(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    status_code::derive_status_code(input)
}
