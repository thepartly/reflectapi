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

impl ToTokens for TokenizableType<'_> {
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

impl ToTokens for TokenizableTypeReference<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let arguments = self.inner.arguments().map(TokenizableTypeReference::new);
        tokens.extend(quote::quote! {
            reflectapi::TypeReference::new(#name, vec![#(#arguments),*])
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

impl ToTokens for TokenizableTypeParameter<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let description = self.inner.description.as_str();
        tokens.extend(quote::quote! {
            reflectapi::TypeParameter::new(#name.into(), #description.into())
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

impl ToTokens for TokenizableField<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let deprecation_note = tokenize_option_string(&self.inner.deprecation_note);
        let deprecated_in = tokenize_option_string(&self.inner.deprecated_in);
        let deprecated_since = tokenize_option_string(&self.inner.deprecated_since);
        let sunset = tokenize_option_string(&self.inner.sunset);
        let type_ref = TokenizableTypeReference::new(&self.inner.type_ref);
        let required = self.inner.required;
        let flattened = self.inner.flattened;
        let hidden = self.inner.hidden;
        let serialize_with = self.inner.serialize_with;
        let deserialize_with = self.inner.deserialize_with;
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
            {
                let mut __reflectapi_field = reflectapi::Field::new(#name.into(), #type_ref);
                __reflectapi_field.serde_name = #serde_name.into();
                __reflectapi_field.description = #description.into();
                __reflectapi_field.deprecation_note = #deprecation_note;
                __reflectapi_field.deprecated_in = #deprecated_in;
                __reflectapi_field.deprecated_since = #deprecated_since;
                __reflectapi_field.sunset = #sunset;
                __reflectapi_field.required = #required;
                __reflectapi_field.flattened = #flattened;
                __reflectapi_field.hidden = #hidden;
                __reflectapi_field.serialize_with = #serialize_with;
                __reflectapi_field.deserialize_with = #deserialize_with;
                __reflectapi_field.transform_callback_fn = #transform_callback_fn;
                __reflectapi_field
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

impl ToTokens for TokenizableVariant<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let deprecation_note = tokenize_option_string(&self.inner.deprecation_note);
        let deprecated_in = tokenize_option_string(&self.inner.deprecated_in);
        let deprecated_since = tokenize_option_string(&self.inner.deprecated_since);
        let sunset = tokenize_option_string(&self.inner.sunset);
        let fields = self.inner.fields().map(TokenizableField::new);

        let fields = match self.inner.fields {
            reflectapi_schema::Fields::Named(..) => quote::quote! {
                reflectapi::Fields::Named(vec![#(#fields),*])
            },
            reflectapi_schema::Fields::Unnamed(..) => quote::quote! {
                reflectapi::Fields::Unnamed(vec![#(#fields),*])
            },
            reflectapi_schema::Fields::None => quote::quote! {
                reflectapi::Fields::None
            },
        };
        let discriminant = self
            .inner
            .discriminant
            .as_ref()
            .map(|d| quote::quote! { Some(#d) })
            .unwrap_or_else(|| quote::quote! { None });
        let untagged = self.inner.untagged;
        let other = self.inner.other;
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_variant = reflectapi::Variant::new(#name.into());
                __reflectapi_variant.serde_name = #serde_name.into();
                __reflectapi_variant.description = #description.into();
                __reflectapi_variant.deprecation_note = #deprecation_note;
                __reflectapi_variant.deprecated_in = #deprecated_in;
                __reflectapi_variant.deprecated_since = #deprecated_since;
                __reflectapi_variant.sunset = #sunset;
                __reflectapi_variant.fields = #fields;
                __reflectapi_variant.discriminant = #discriminant;
                __reflectapi_variant.untagged = #untagged;
                __reflectapi_variant.other = #other;
                __reflectapi_variant
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

impl ToTokens for TokenizableRepresentation<'_> {
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

impl ToTokens for TokenizableEnum<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self.inner.parameters().map(TokenizableTypeParameter::new);
        let representation = TokenizableRepresentation::new(&self.inner.representation);
        let open = self.inner.open;
        let deny_unknown_fields = self.inner.deny_unknown_fields;
        let variants = self.inner.variants().map(TokenizableVariant::new);
        let codegen_config =
            TokenizableLanguageSpecificTypeCodegenConfig(&self.inner.codegen_config);
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_enum = reflectapi::Enum::new(#name.into());
                __reflectapi_enum.serde_name = #serde_name.into();
                __reflectapi_enum.description = #description.into();
                __reflectapi_enum.parameters = vec![#(#parameters),*];
                __reflectapi_enum.representation = #representation;
                __reflectapi_enum.open = #open;
                __reflectapi_enum.deny_unknown_fields = #deny_unknown_fields;
                __reflectapi_enum.variants = vec![#(#variants),*];
                __reflectapi_enum.codegen_config = #codegen_config;
                __reflectapi_enum
            }
        });
    }
}

pub(crate) struct TokenizableLanguageSpecificTypeCodegenConfig<'a>(
    &'a reflectapi_schema::LanguageSpecificTypeCodegenConfig,
);

impl ToTokens for TokenizableLanguageSpecificTypeCodegenConfig<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let rust = TokenizableRustTypeCodegenConfig(&self.0.rust);
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_codegen_config =
                    reflectapi::LanguageSpecificTypeCodegenConfig::default();
                __reflectapi_codegen_config.rust = #rust;
                __reflectapi_codegen_config
            }
        });
    }
}

pub(crate) struct TokenizableRustTypeCodegenConfig<'a>(&'a RustTypeCodegenConfig);

impl ToTokens for TokenizableRustTypeCodegenConfig<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let additional_derives = &self.0.additional_derives;
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_rust_config = reflectapi::RustTypeCodegenConfig::default();
                __reflectapi_rust_config.additional_derives =
                    std::collections::BTreeSet::from_iter([#(String::from(#additional_derives)),*]);
                __reflectapi_rust_config
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

impl ToTokens for TokenizableStruct<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let serde_name = self.inner.serde_name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self.inner.parameters().map(TokenizableTypeParameter::new);
        let fields = self.inner.fields().map(TokenizableField::new);
        let fields = match self.inner.fields {
            reflectapi_schema::Fields::Named(..) => quote::quote! {
                reflectapi::Fields::Named(vec![#(#fields),*])
            },
            reflectapi_schema::Fields::Unnamed(..) => quote::quote! {
                reflectapi::Fields::Unnamed(vec![#(#fields),*])
            },
            reflectapi_schema::Fields::None => quote::quote! {
                reflectapi::Fields::None
            },
        };

        let transparent = self.inner.transparent;
        let distinct = self.inner.distinct;
        let deny_unknown_fields = self.inner.deny_unknown_fields;
        let codegen_config =
            TokenizableLanguageSpecificTypeCodegenConfig(&self.inner.codegen_config);
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_struct = reflectapi::Struct::new(#name);
                __reflectapi_struct.serde_name = #serde_name.into();
                __reflectapi_struct.description = #description.into();
                __reflectapi_struct.parameters = vec![#(#parameters),*];
                __reflectapi_struct.fields = #fields;
                __reflectapi_struct.transparent = #transparent;
                __reflectapi_struct.distinct = #distinct;
                __reflectapi_struct.deny_unknown_fields = #deny_unknown_fields;
                __reflectapi_struct.codegen_config = #codegen_config;
                __reflectapi_struct
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

impl ToTokens for TokenizablePrimitive<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.inner.name.as_str();
        let description = self.inner.description.as_str();
        let parameters = self.inner.parameters().map(TokenizableTypeParameter::new);
        let fallback = match &self.inner.fallback {
            Some(fallback) => {
                let fallback = TokenizableTypeReference::new(fallback);
                quote::quote! { Some(#fallback) }
            }
            None => quote::quote! { None },
        };
        let format = tokenize_option_string(&self.inner.format);
        let codegen_config =
            TokenizableLanguageSpecificTypeCodegenConfig(&self.inner.codegen_config);
        tokens.extend(quote::quote! {
            {
                let mut __reflectapi_primitive = reflectapi::Primitive::new(
                    #name.into(),
                    #description.into(),
                    vec![#(#parameters),*],
                    #fallback,
                );
                __reflectapi_primitive.format = #format;
                __reflectapi_primitive.codegen_config = #codegen_config;
                __reflectapi_primitive
            }
        });
    }
}

fn tokenize_option_string(value: &Option<String>) -> proc_macro2::TokenStream {
    match value {
        Some(value) => quote::quote! { Some(#value.into()) },
        None => quote::quote! { None },
    }
}
