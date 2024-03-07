use quote::ToTokens;
use reflect_schema::*;

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
                    reflect::Type::Enum(#tks)
                });
            }
            Type::Struct(s) => {
                let tks = TokenizableStruct::new(s);
                tokens.extend(quote::quote! {
                    reflect::Type::Struct(#tks)
                });
            }
            Type::Primitive(p) => {
                let tks = TokenizablePrimitive::new(p);
                tokens.extend(quote::quote! {
                    reflect::Type::Primitive(#tks)
                });
            }
            Type::Alias(a) => {
                let tks = TokenizableAlias::new(a);
                tokens.extend(quote::quote! {
                    reflect::Type::Alias(#tks)
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
            .map(|p| TokenizableTypeReference::new(p));
        tokens.extend(quote::quote! {
            reflect::TypeReference {
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
            reflect::TypeParameter {
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
        let description = self.inner.description.as_str();
        let type_ref = TokenizableTypeReference::new(&self.inner.type_ref);
        let required = self.inner.required;
        let flattened = self.inner.flattened;
        tokens.extend(quote::quote! {
            reflect::Field {
                name: #name.into(),
                description: #description.into(),
                type_ref: #type_ref,
                required: #required,
                flattened: #flattened,
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
        let description = self.inner.description.as_str();
        let fields = self.inner.fields().map(|f| TokenizableField::new(f));
        let discriminant = self.inner.discriminant;
        tokens.extend(quote::quote! {
            reflect::Variant {
                name: #name.into(),
                description: #description.into(),
                fields: vec![#(#fields),*],
                discriminant: #discriminant,
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
            Representation::String => {
                quote::quote! {
                    reflect::Representation::String
                }
            }
            Representation::I8 => {
                quote::quote! {
                    reflect::Representation::I8
                }
            }
            Representation::U8 => {
                quote::quote! {
                    reflect::Representation::U8
                }
            }
            Representation::U16 => {
                quote::quote! {
                    reflect::Representation::U16
                }
            }
            Representation::U32 => {
                quote::quote! {
                    reflect::Representation::U32
                }
            }
            Representation::U64 => {
                quote::quote! {
                    reflect::Representation::U64
                }
            }
            Representation::U128 => {
                quote::quote! {
                    reflect::Representation::U128
                }
            }
            Representation::Usize => {
                quote::quote! {
                    reflect::Representation::Usize
                }
            }
            Representation::I16 => {
                quote::quote! {
                    reflect::Representation::I16
                }
            }
            Representation::I32 => {
                quote::quote! {
                    reflect::Representation::I32
                }
            }
            Representation::I64 => {
                quote::quote! {
                    reflect::Representation::I64
                }
            }
            Representation::I128 => {
                quote::quote! {
                    reflect::Representation::I128
                }
            }
            Representation::Isize => {
                quote::quote! {
                    reflect::Representation::Isize
                }
            }
            Representation::Untagged => {
                quote::quote! {
                    reflect::Representation::Untagged
                }
            }
            Representation::InnerTagged(tag) => {
                quote::quote! {
                    reflect::Representation::InnerTagged(#tag)
                }
            }
            Representation::OuterTagged { tag, content } => {
                quote::quote! {
                    reflect::Representation::OuterTagged {
                        tag: #tag,
                        content: #content,
                    }
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
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(|p| TokenizableTypeParameter::new(p));
        let representation = TokenizableRepresentation::new(&self.inner.representation);
        let variants = self.inner.variants().map(|v| TokenizableVariant::new(v));
        tokens.extend(quote::quote! {
            reflect::Enum {
                name: #name.into(),
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
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(|p| TokenizableTypeParameter::new(p));
        let fields = self.inner.fields().map(|f| TokenizableField::new(f));
        tokens.extend(quote::quote! {
            reflect::Struct {
                name: #name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
                fields: vec![#(#fields),*],
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
            .map(|p| TokenizableTypeParameter::new(p));
        tokens.extend(quote::quote! {
            reflect::Primitive {
                name: #name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
            }
        });
    }
}

pub(crate) struct TokenizableAlias<'a> {
    pub inner: &'a Alias,
}

impl<'a> TokenizableAlias<'a> {
    pub fn new(inner: &'a Alias) -> Self {
        TokenizableAlias { inner }
    }
}

impl<'a> ToTokens for TokenizableAlias<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self
            .inner
            .parameters()
            .map(|p| TokenizableTypeParameter::new(p));
        let type_ref = TokenizableTypeReference::new(&self.inner.type_ref);
        tokens.extend(quote::quote! {
            reflect::Alias {
                name: #name.into(),
                description: #description.into(),
                parameters: vec![#(#parameters),*],
                type_ref: #type_ref,
            }
        });
    }
}
