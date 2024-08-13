extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(Serialize, attributes(serde))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_derive(Deserialize, attributes(serde))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    TokenStream::new()
}
