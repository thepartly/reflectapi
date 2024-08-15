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
    schemas: BTreeMap<String, CompositeSchema>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RequestBody {
    content: BTreeMap<String, MediaType>,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaType {
    schema: Ref<Schema>,
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
    },
    Object {
        properties: BTreeMap<String, InlineOrRef<Schema>>,
    },
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum CompositeSchema {
    OneOf {
        #[serde(rename = "oneOf")]
        subschemas: Vec<InlineOrRef<Schema>>,
    },
    Schema(Schema),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Schema {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(flatten)]
    pub ty: Type,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(Box<T>),
    Many(Vec<T>),
}

impl<T> From<T> for OneOrMany<T> {
    fn from(one: T) -> Self {
        OneOrMany::One(Box::new(one))
    }
}

impl<T> From<Vec<T>> for OneOrMany<T> {
    fn from(many: Vec<T>) -> Self {
        OneOrMany::Many(many)
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
