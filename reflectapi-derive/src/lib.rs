mod context;
mod derive;
mod parser;
mod symbol;
mod tokenizable_schema;

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Input, attributes(reflect, serde))]
pub fn derive_reflectapi_input(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Output, attributes(reflect, serde))]
pub fn derive_reflectapi_output(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input, context::ReflectType::Output)
}
