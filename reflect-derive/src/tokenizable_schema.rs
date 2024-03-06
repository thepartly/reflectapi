use quote::ToTokens;
use reflect_schema::Type;

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
