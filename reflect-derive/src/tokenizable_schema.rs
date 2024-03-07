use quote::ToTokens;
use reflect_schema::{Type, TypeReference};

// pub(crate) struct TokenizableSchema {
//     pub inner: Schema,
// }

// impl TokenizableSchema {
//     pub fn new(inner: Schema) -> Self {
//         TokenizableSchema { inner }
//     }
// }

// impl ToTokens for TokenizableSchema {
//     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
//         let schema = self.inner.to_json();
//         tokens.extend(quote::quote! {
//             reflect::Schema::from_json(#schema)
//         });
//     }
// }

pub(crate) struct TokenizableType<'a> {
    pub inner: &'a Type,
}

impl<'a> TokenizableType<'a> {
    pub fn new(inner: &'a Type) -> Self {
        TokenizableType { inner }
    }
}

impl<'a> ToTokens for TokenizableType<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let schema = self.inner.to_json();
        tokens.extend(quote::quote! {
            reflect::Type::from_json(#schema)
        });
    }
}

pub(crate) struct TokenizableTypeReference<'a> {
    pub inner: &'a TypeReference,
}

impl<'a> TokenizableTypeReference<'a> {
    pub fn new(inner: &'a TypeReference) -> Self {
        TokenizableTypeReference { inner }
    }
}

impl<'a> ToTokens for TokenizableTypeReference<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let parameters = self.inner.parameters().map(|p| TokenizableTypeReference::new(p));
        tokens.extend(quote::quote! {
            reflect::TypeReference::new(#name.into(), vec![#(#parameters),*])
        });
    }
}
