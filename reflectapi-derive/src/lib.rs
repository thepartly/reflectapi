mod context;
mod derive;
mod parser;
mod status_code;
mod symbol;
mod tokenizable_schema;

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Input, attributes(reflectapi, serde))]
pub fn derive_reflectapi_input(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Output, attributes(reflectapi, serde))]
pub fn derive_reflectapi_output(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Output)
}

/// Implements `reflectapi::StatusCode` for an error enum. Each variant has its
/// status, `#[reflectapi(status = 404)]`, or takes the status and headers of
/// its one field, `#[reflectapi(transparent)]`.
#[proc_macro_derive(StatusCode, attributes(reflectapi))]
pub fn derive_reflectapi_status_code(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    status_code::derive_status_code(input)
}
