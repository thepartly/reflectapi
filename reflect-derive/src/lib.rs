mod derive;
mod symbol;
mod tokenizable_schema;

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Reflect, attributes(reflect, serde))]
pub fn derive_reflect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_reflect(input)
}
