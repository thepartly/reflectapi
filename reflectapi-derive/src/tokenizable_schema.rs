use quote::ToTokens;
use reflectapi_schema::*;

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
        match self.inner {
            Type::Enum(e) => {
                let tks = TokenizableEnum::new(e);
                tokens.extend(quote::quote! {
                    reflectapi::Type::Enum(#tks)
                });
            }
            Type::Struct(s) => {
                let tks = TokenizableStruct::new(s);
                tokens.extend(quote::quote! {
                    reflectapi::Type::Struct(#tks)
                });
            }
            Type::Primitive(p) => {
                let tks = TokenizablePrimitive::new(p);
                tokens.extend(quote::quote! {
                    reflectapi::Type::Primitive(#tks)
                });
            }
        }
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
        let parameters = self
            .inner
            .parameters()
            .map(TokenizableTypeReference::new);
        tokens.extend(quote::quote! {
            reflectapi::TypeReference {
                name: #name.into(),
                parameters: vec![#(#parameters),*]
            }
        });
    }
}

pub(crate) struct TokenizableTypeParameter<'a> {
    pub inner: &'a TypeParameter,
}

impl<'a> TokenizableTypeParameter<'a> {
    pub fn new(inner: &'a TypeParameter) -> Self {
        TokenizableTypeParameter { inner }
    }
}

impl<'a> ToTokens for TokenizableTypeParameter<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let description = self.inner.description.as_str();
        tokens.extend(quote::quote! {
            reflectapi::TypeParameter {
                name: #name.into(),
                description: #description.into()
            }
        });
    }
}

pub(crate) struct TokenizableField<'a> {
    pub inner: &'a Field,
}

impl<'a> TokenizableField<'a> {
    pub fn new(inner: &'a Field) -> Self {
        TokenizableField { inner }
    }
}

impl<'a> ToTokens for TokenizableField<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let type_ref = TokenizableTypeReference::new(&self.inner.type_ref);
        let required = self.inner.required;
        let flattened = self.inner.flattened;
        let transform_callback = self.inner.transform_callback.as_str();
        let mut transform_callback_fn = quote::quote! {
            None
        };
        if !transform_callback.is_empty() {
            let transformation_fn_path =
                syn::parse_str::<proc_macro2::TokenStream>(transform_callback).unwrap();
            transform_callback_fn = quote::quote! {
                Some(#transformation_fn_path)
            };
        }
        tokens.extend(quote::quote! {
            reflectapi::Field {
                name: #name.into(),
                serde_name: #serde_name.into(),
                description: #description.into(),
                type_ref: #type_ref,
                required: #required,
                flattened: #flattened,
                transform_callback: String::new(),
                transform_callback_fn: #transform_callback_fn,
            }
        });
    }
}

pub(crate) struct TokenizableVariant<'a> {
    pub inner: &'a Variant,
}

impl<'a> TokenizableVariant<'a> {
    pub fn new(inner: &'a Variant) -> Self {
        TokenizableVariant { inner }
    }
}

impl<'a> ToTokens for TokenizableVariant<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let fields = self.inner.fields().map(TokenizableField::new);
        let discriminant = self
            .inner
            .discriminant
            .as_ref()
            .map(|d| quote::quote! { Some(#d) })
            .unwrap_or_else(|| quote::quote! { None });
        let untagged = self.inner.untagged;
        tokens.extend(quote::quote! {
            reflectapi::Variant {
                name: #name.into(),
                serde_name: #serde_name.into(),
                description: #description.into(),
                fields: vec![#(#fields),*],
                discriminant: #discriminant,
                untagged: #untagged,
            }
        });
    }
}

pub(crate) struct TokenizableRepresentation<'a> {
    pub inner: &'a Representation,
}

impl<'a> TokenizableRepresentation<'a> {
    pub fn new(inner: &'a Representation) -> Self {
        TokenizableRepresentation { inner }
    }
}

impl<'a> ToTokens for TokenizableRepresentation<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let tks = match self.inner {
            Representation::External => {
                quote::quote! {
                    reflectapi::Representation::External
                }
            }
            Representation::Internal { tag } => {
                quote::quote! {
                    reflectapi::Representation::Internal { tag: #tag.into() }
                }
            }
            Representation::Adjacent { tag, content } => {
                quote::quote! {
                    reflectapi::Representation::Adjacent { tag: #tag.into(), content: #content.into() }
                }
            }
            Representation::None => {
                quote::quote! {
                    reflectapi::Representation::None
                }
            }
        };
        tokens.extend(tks);
    }
}

pub(crate) struct TokenizableEnum<'a> {
    pub inner: &'a Enum,
}

impl<'a> TokenizableEnum<'a> {
    pub fn new(inner: &'a Enum) -> Self {
        TokenizableEnum { inner }
    }
}

impl<'a> ToTokens for TokenizableEnum<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(TokenizableTypeParameter::new);
        let representation = TokenizableRepresentation::new(&self.inner.representation);
        let variants = self.inner.variants().map(TokenizableVariant::new);
        tokens.extend(quote::quote! {
            reflectapi::Enum {
                name: #name.into(),
                serde_name: #serde_name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
                representation: #representation,
                variants: vec![#(#variants),*],
            }
        });
    }
}

pub(crate) struct TokenizableStruct<'a> {
    pub inner: &'a Struct,
}

impl<'a> TokenizableStruct<'a> {
    pub fn new(inner: &'a Struct) -> Self {
        TokenizableStruct { inner }
    }
}

impl<'a> ToTokens for TokenizableStruct<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(TokenizableTypeParameter::new);
        let fields = self.inner.fields().map(TokenizableField::new);
        let transparent = self.inner.transparent;
        tokens.extend(quote::quote! {
            reflectapi::Struct {
                name: #name.into(),
                serde_name: #serde_name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
                fields: vec![#(#fields),*],
                transparent: #transparent,
            }
        });
    }
}

pub(crate) struct TokenizablePrimitive<'a> {
    pub inner: &'a Primitive,
}

impl<'a> TokenizablePrimitive<'a> {
    pub fn new(inner: &'a Primitive) -> Self {
        TokenizablePrimitive { inner }
    }
}

impl<'a> ToTokens for TokenizablePrimitive<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(TokenizableTypeParameter::new);
        let fallback = self
            .inner
            .fallback
            .as_ref()
            .map(TokenizableTypeReference::new);
        tokens.extend(quote::quote! {
            reflectapi::Primitive {
                name: #name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
                fallback: #fallback,
            }
        });
    }
}
