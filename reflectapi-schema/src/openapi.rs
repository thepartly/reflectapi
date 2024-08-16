//! Subset of definitions of OpenAPI 3.1.0 spec.
//! These definitions are only suitable only for serialization, not deserialization
//! because they are stricter than the OpenAPI spec.

mod convert;

use std::collections::BTreeMap;
use std::marker::PhantomData;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spec {
    pub openapi: String,
    pub info: Info,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub paths: BTreeMap<String, PathItem>,
    pub components: Components,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub title: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Components {
    schemas: BTreeMap<String, Schema>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PathItem {
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    get: Option<Operation>,
    post: Operation,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    operation_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_body: Option<RequestBody>,
    responses: BTreeMap<String, Response>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RequestBody {
    content: BTreeMap<String, MediaType>,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Response {
    description: String,
    // headers: BTreeMap<String, InlineOrRef<Schema>>,
    content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaType {
    schema: InlineOrRef<Schema>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Boolean,
    Integer,
    Number,
    String,
    Array {
        items: Box<InlineOrRef<Schema>>,
        #[serde(rename = "uniqueItems")]
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        unique_items: bool,
    },
    #[serde(rename = "array")]
    Tuple {
        #[serde(rename = "prefixItems")]
        prefix_items: Vec<InlineOrRef<Schema>>,
    },
    Object {
        properties: BTreeMap<String, InlineOrRef<Schema>>,
    },
    #[serde(rename = "object")]
    Map {
        /// The key type is implicitly assumed to be string as OpenAPI does not support other key types.
        /// The value below should be the value schema.
        #[serde(rename = "additionalProperties")]
        additional_properties: Box<InlineOrRef<Schema>>,
    },
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum Schema {
    OneOf {
        #[serde(rename = "oneOf")]
        subschemas: Vec<InlineOrRef<Schema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        discriminator: Option<Discriminator>,
    },
    Flat(FlatSchema),
}

impl From<FlatSchema> for Schema {
    fn from(schema: FlatSchema) -> Self {
        Schema::Flat(schema)
    }
}

impl Schema {
    pub fn empty_object() -> Self {
        Schema::Flat(FlatSchema::empty_object())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discriminator {
    /// Defines a discriminator property name which must be found within all composite objects.
    pub property_name: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FlatSchema {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(flatten)]
    pub ty: Type,
}

impl FlatSchema {
    pub fn empty_object() -> Self {
        Self {
            description: "empty object".to_owned(),
            ty: Type::Object {
                properties: BTreeMap::new(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InlineOrRef<T> {
    Inline(T),
    Ref(Ref<T>),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Ref<T> {
    #[serde(rename = "$ref")]
    pub ref_path: String,
    // #[expect(unused)]
    #[serde(skip)]
    phantom: PhantomData<T>,
}

impl<T> Ref<T> {
    pub fn new(ref_path: impl Into<String>) -> Self {
        Self {
            ref_path: ref_path.into(),
            phantom: PhantomData,
        }
    }
}
