use quote::ToTokens;
use reflect_schema::Schema;

pub(crate) struct TokenizableSchema {
    pub schema: Schema,
}

impl TokenizableSchema {
    pub fn new(schema: Schema) -> Self {
        TokenizableSchema { schema }
    }
}

impl ToTokens for TokenizableSchema {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let schema = self.schema.to_json();
        tokens.extend(quote::quote! {
            reflect::Schema::from_json(#schema)
        });
    }
}
