//! Convert `reflectapi.json` to a approximately equivalent `openapi.json` file.
// `reflectapi` constructs within this file are always prefixed with `crate::` to avoid confusion
// with `openapi` constructs of the same name.
//
// Since `reflectapi` has a richer type system than `openapi`, the conversion is not perfect.
// Generics are effectively monomorphized i.e. each instantiation of a generic type is inlined.
//
// Also contains a subset of definitions of OpenAPI 3.1.0 spec.
// These definitions are only suitable only for serialization, not deserialization
// because they are stricter than the OpenAPI spec.

use core::fmt;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::OnceLock;

use reflectapi_schema::{Instantiate, Substitute};
use serde::Serialize;

use super::Config;

pub fn generate(schema: crate::Schema, _config: &Config) -> anyhow::Result<String> {
    let spec = Spec::from(&schema);
    Ok(serde_json::to_string_pretty(&spec)?)
}

impl From<&crate::Schema> for Spec {
    fn from(schema: &crate::Schema) -> Self {
        Converter::default().convert(schema)
    }
}

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Parameter {
    name: String,
    #[serde(rename = "in")]
    location: In,
    required: bool,
    schema: InlineOrRef<Schema>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum In {
    Header,
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
    String {
        /// `None` indicates an unrestricted string. `Some([])` indicates an empty type.
        #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
        values: Option<Vec<String>>,
    },
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
        #[serde(skip_serializing_if = "String::is_empty")]
        title: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        required: Vec<String>,
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

#[derive(Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum Schema {
    AllOf {
        #[serde(rename = "allOf")]
        subschemas: Vec<InlineOrRef<Schema>>,
    },
    OneOf {
        #[serde(rename = "oneOf")]
        subschemas: Vec<InlineOrRef<Schema>>,
    },
    Flat(FlatSchema),
}

impl fmt::Debug for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Schema::AllOf { subschemas } => f
                .debug_struct("AllOf")
                .field("subschemas", subschemas)
                .finish(),
            Schema::OneOf { subschemas } => f
                .debug_struct("OneOf")
                .field("subschemas", subschemas)
                .finish(),
            Schema::Flat(schema) => schema.fmt(f),
        }
    }
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

    pub fn empty_tuple() -> Self {
        Schema::Flat(FlatSchema::empty_tuple())
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
                title: "".to_owned(),
                required: vec![],
                properties: BTreeMap::new(),
            },
        }
    }

    pub fn empty_tuple() -> Self {
        Self {
            description: "empty tuple".to_owned(),
            ty: Type::Tuple {
                prefix_items: vec![],
            },
        }
    }
}

#[derive(Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InlineOrRef<T> {
    Inline(T),
    Ref(Ref<T>),
}

impl<T: fmt::Debug> fmt::Debug for InlineOrRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InlineOrRef::Inline(inner) => inner.fmt(f),
            InlineOrRef::Ref(r) => r.fmt(f),
        }
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub struct Ref<T> {
    #[serde(rename = "$ref")]
    pub ref_path: String,
    // #[expect(unused)]
    #[serde(skip)]
    phantom: PhantomData<T>,
}

impl<T> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ref_path.fmt(f)
    }
}

impl<T> Ref<T> {
    pub fn new(ref_path: impl Into<String>) -> Self {
        Self {
            ref_path: ref_path.into(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
struct Converter {
    components: Components,
}

impl Converter {
    fn convert(mut self, schema: &crate::Schema) -> Spec {
        Spec {
            openapi: "3.1.0".into(),
            info: Info {
                title: schema.name.clone(),
                description: schema.description.clone(),
                version: "1.0.0".into(),
            },
            paths: schema
                .functions
                .iter()
                .map(|f| {
                    (
                        format!("{}/{}", f.path(), f.name()),
                        self.convert_function(schema, f),
                    )
                })
                .collect(),
            components: self.components,
        }
    }

    fn convert_function(&mut self, schema: &crate::Schema, f: &crate::Function) -> PathItem {
        let operation = Operation {
            operation_id: f.name.clone(),
            description: f.description.clone(),
            parameters: f
                .input_headers()
                .map_or_else(Vec::new, |headers| self.convert_headers(schema, headers)),
            request_body: f.input_type().map(|ty| RequestBody {
                content: BTreeMap::from([(
                    // TODO msgpack
                    "application/json".to_owned(),
                    MediaType {
                        schema: self.convert_type_ref(schema, Kind::Input, ty),
                    },
                )]),
                required: true,
            }),
            responses: BTreeMap::from([(
                "200".into(),
                Response {
                    description: "200 OK".to_owned(),
                    // TODO msgpack
                    content: BTreeMap::from([(
                        "application/json".to_owned(),
                        MediaType {
                            schema: f.output_type().map_or_else(
                                || InlineOrRef::Inline(Schema::empty_object()),
                                |ty| self.convert_type_ref(schema, Kind::Output, ty),
                            ),
                        },
                    )]),
                },
            )]),
        };

        PathItem {
            description: f.description.clone(),
            get: None,
            // If we do this the `operation_id` will not be unique
            // get: f.readonly.then(|| operation.clone()),
            post: operation,
        }
    }

    fn resolve_schema_ref<'a>(&'a self, r: &'a InlineOrRef<Schema>) -> &'a Schema {
        match r {
            InlineOrRef::Inline(schema) => schema,
            InlineOrRef::Ref(r) => self
                .components
                .schemas
                .get(r.ref_path.strip_prefix("#/components/schemas/").unwrap())
                .unwrap(),
        }
    }

    fn convert_headers(
        &mut self,
        schema: &crate::Schema,
        type_ref: &crate::TypeReference,
    ) -> Vec<Parameter> {
        let schema = match self.convert_type_ref(schema, Kind::Input, type_ref) {
            InlineOrRef::Inline(schema) => schema,
            InlineOrRef::Ref(r) => {
                let name = r.ref_path.strip_prefix("#/components/schemas/").unwrap();
                self.components.schemas.remove(name).unwrap()
            }
        };

        match schema {
            Schema::Flat(FlatSchema {
                ty:
                    Type::Object {
                        title: _,
                        properties,
                        required,
                    },
                ..
            }) => properties
                .into_iter()
                .map(|(name, schema)| {
                    let required = required.contains(&name);
                    Parameter {
                        name,
                        location: In::Header,
                        required,
                        schema,
                    }
                })
                .collect(),
            _ => unreachable!("header type is a struct"),
        }
    }

    fn convert_type_ref(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        ty_ref: &crate::TypeReference,
    ) -> InlineOrRef<Schema> {
        let name = normalize(&ty_ref.name);
        let schema_ref = Ref::new(format!("#/components/schemas/{name}"));
        if self.components.schemas.contains_key(&name) {
            return InlineOrRef::Ref(schema_ref);
        }

        let reflect_ty = match kind {
            Kind::Input => schema.input_types.get_type(&ty_ref.name),
            Kind::Output => schema.output_types.get_type(&ty_ref.name),
        }
        .unwrap_or_else(|| {
            if ty_ref.name == "std::string::String" {
                // HACK: string type is used for the tag type, but may not exist in the schema.
                static STRING_TYPE: OnceLock<crate::Type> = OnceLock::new();
                return STRING_TYPE.get_or_init(|| {
                    crate::Type::Primitive(crate::Primitive {
                        name: "std::string::String".into(),
                        description: "UTF-8 encoded string".into(),
                        parameters: vec![],
                        fallback: None,
                    })
                });
            };

            panic!("{kind:?} type not found: {ty_ref:?}")
        });

        let stub = Schema::Flat(FlatSchema {
            description: "stub".into(),
            ty: Type::Null,
        });

        if !matches!(reflect_ty, crate::Type::Primitive(_))
            && reflect_ty.parameters().next().is_none()
        {
            // Insert a stub definition so recursive types don't get stuck.
            self.components.schemas.insert(name.clone(), stub.clone());
        }

        let schema = match reflect_ty {
            crate::Type::Struct(strukt) => match self.struct_to_schema(
                schema,
                kind,
                ty_ref,
                &strukt.clone().instantiate(&ty_ref.arguments),
            ) {
                InlineOrRef::Inline(schema) => schema,
                InlineOrRef::Ref(r) => {
                    self.components.schemas.remove(&name);
                    return InlineOrRef::Ref(r);
                }
            },
            crate::Type::Enum(adt) => {
                if adt.name == "std::option::Option" || adt.name == "reflectapi::Option" {
                    // Special case `Option` to generate a nicer spec.
                    Schema::OneOf {
                        subschemas: vec![
                            InlineOrRef::Inline(Schema::Flat(FlatSchema {
                                description: "Null".to_owned(),
                                ty: Type::Null,
                            })),
                            self.convert_type_ref(schema, kind, ty_ref.arguments().next().unwrap()),
                        ],
                    }
                } else {
                    match self.enum_to_schema(
                        schema,
                        kind,
                        ty_ref,
                        &adt.clone().instantiate(&ty_ref.arguments),
                    ) {
                        InlineOrRef::Inline(schema) => schema,
                        InlineOrRef::Ref(r) => {
                            self.components.schemas.remove(&name);
                            return InlineOrRef::Ref(r);
                        }
                    }
                }
            }
            crate::Type::Primitive(prim) => {
                assert_eq!(
                    ty_ref.arguments.len(),
                    prim.parameters.len(),
                    "primitive type with wrong number of arguments: {ty_ref:?}"
                );
                let ty = match prim.name() {
                    "f32" | "f64" => Type::Number,
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32"
                    | "u64" | "u128" | "usize" => Type::Integer,
                    "bool" => Type::Boolean,
                    "serde_json::Value" => Type::Object {
                        // Not sure if there is a better way to represent this
                        title: "serde_json::Value".to_owned(),
                        required: vec![],
                        properties: BTreeMap::new(),
                    },
                    "uuid::Uuid" | "char" | "std::string::String" => Type::String { values: None },
                    "std::marker::PhantomData" | "std::tuple::Tuple0" => Type::Null,
                    "std::vec::Vec" | "std::array::Array" | "std::collections::HashSet" => {
                        Type::Array {
                            items: Box::new(self.convert_type_ref(
                                schema,
                                kind,
                                ty_ref.arguments().next().unwrap(),
                            )),
                            unique_items: prim.name().ends_with("Set"),
                        }
                    }
                    // There is no way to specify the key type in OpenAPI, so we assume it's always a string (unenforced).
                    "std::collections::HashMap" => Type::Map {
                        additional_properties: Box::new(self.convert_type_ref(
                            schema,
                            kind,
                            ty_ref.arguments().last().unwrap(),
                        )),
                    },
                    "std::tuple::Tuple1"
                    | "std::tuple::Tuple2"
                    | "std::tuple::Tuple3"
                    | "std::tuple::Tuple4"
                    | "std::tuple::Tuple5"
                    | "std::tuple::Tuple6"
                    | "std::tuple::Tuple7"
                    | "std::tuple::Tuple8"
                    | "std::tuple::Tuple9"
                    | "std::tuple::Tuple10"
                    | "std::tuple::Tuple11"
                    | "std::tuple::Tuple12" => Type::Tuple {
                        prefix_items: ty_ref
                            .arguments()
                            .map(|arg| self.convert_type_ref(schema, kind, arg))
                            .collect(),
                    },
                    name => {
                        if let Some(fallback) = prim.fallback() {
                            let subst =
                                reflectapi_schema::mk_subst(&prim.parameters, &ty_ref.arguments);
                            return self.convert_type_ref(
                                schema,
                                kind,
                                &fallback.clone().subst(&subst),
                            );
                        } else {
                            unimplemented!("primitive with no fallback: {name}")
                        }
                    }
                };

                Schema::Flat(FlatSchema {
                    description: reflect_ty.description().to_owned(),
                    ty,
                })
            }
        };

        // Inline if the type is generic, otherwise store as a component schema.
        if ty_ref.arguments.is_empty() {
            if let Some(existing) = self.components.schemas.insert(name.clone(), schema) {
                assert_eq!(
                    existing, stub,
                    "overwrote a schema that wasn't a stub: name={name}"
                );
            }

            InlineOrRef::Ref(schema_ref)
        } else {
            InlineOrRef::Inline(schema)
        }
    }

    fn enum_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        type_ref: &crate::TypeReference,
        adt: &crate::Enum,
    ) -> InlineOrRef<Schema> {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        let tags = match adt.representation() {
            reflectapi_schema::Representation::External
            | reflectapi_schema::Representation::Internal { .. }
            | reflectapi_schema::Representation::Adjacent { .. } => Some(
                adt.variants()
                    .map(|v| v.serde_name().to_owned())
                    .collect::<Vec<_>>(),
            ),
            reflectapi_schema::Representation::None => None,
        };

        // Special case enums that are all unit variants
        if matches!(adt.representation(), crate::Representation::External)
            && adt
                .variants()
                .all(|v| matches!(v.fields, crate::Fields::None))
        {
            return InlineOrRef::Inline(Schema::Flat(FlatSchema {
                description: adt.description().to_owned(),
                ty: Type::String { values: tags },
            }));
        }

        let subschemas = adt
            .variants()
            .map(|variant| {
                self.variant_to_schema(schema, kind, type_ref, adt, tags.as_deref(), variant)
            })
            .collect::<Vec<_>>();

        let schema = match adt.representation() {
            crate::Representation::Internal { tag: _ } => Schema::OneOf { subschemas },
            crate::Representation::External
            | crate::Representation::Adjacent { .. }
            | crate::Representation::None => Schema::OneOf { subschemas },
        };

        InlineOrRef::Inline(schema)
    }

    fn variant_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        type_ref: &crate::TypeReference,
        adt: &crate::Enum,
        all_tags: Option<&[String]>,
        variant: &crate::Variant,
    ) -> InlineOrRef<Schema> {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        let type_ref =
            crate::TypeReference::new(variant.name().to_owned(), type_ref.arguments.clone());

        let mut strukt = crate::Struct {
            name: variant.name().to_owned(),
            serde_name: variant.serde_name.to_owned(),
            description: variant.description().to_owned(),
            parameters: vec![],
            fields: variant.fields.clone(),
            transparent: false,
            codegen_config: adt.codegen_config.clone(),
        };

        let repr = match variant.untagged {
            true => &crate::Representation::None,
            false => adt.representation(),
        };

        match repr {
            crate::Representation::External => {
                if variant.fields.is_empty() {
                    // ```rust
                    // #[derive(Serialize)]
                    // enum {
                    //    A,            // "A"
                    //    B {}          // { "B": {} }
                    //    C()           // { "C": [] }
                    // }
                    // ```

                    let ty = match variant.fields {
                        reflectapi_schema::Fields::Named(_) => Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: vec![],
                            properties: BTreeMap::from_iter([(
                                variant.serde_name().to_owned(),
                                InlineOrRef::Inline(Schema::empty_object()),
                            )]),
                        },
                        reflectapi_schema::Fields::Unnamed(_) => Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: vec![],
                            properties: BTreeMap::from_iter([(
                                variant.serde_name().to_owned(),
                                InlineOrRef::Inline(Schema::empty_tuple()),
                            )]),
                        },
                        reflectapi_schema::Fields::None => Type::String {
                            values: Some(vec![variant.serde_name().to_owned()]),
                        },
                    };

                    return InlineOrRef::Inline(
                        FlatSchema {
                            description: variant.description().to_owned(),
                            ty,
                        }
                        .into(),
                    );
                }

                let field = if variant.fields.len() == 1 && !variant.fields[0].is_named() {
                    self.convert_type_ref(schema, kind, variant.fields[0].type_ref())
                } else {
                    self.struct_to_schema(schema, kind, &type_ref, &strukt)
                };

                return InlineOrRef::Inline(
                    FlatSchema {
                        description: variant.description().to_owned(),
                        ty: Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: vec![variant.serde_name().to_owned()],
                            properties: BTreeMap::from([(variant.serde_name().to_owned(), field)]),
                        },
                    }
                    .into(),
                );
            }
            crate::Representation::Adjacent { tag, content } => {
                let data = if variant.fields.len() == 1 && !variant.fields[0].is_named() {
                    self.convert_type_ref(schema, kind, variant.fields[0].type_ref())
                } else {
                    self.struct_to_schema(schema, kind, &type_ref, &strukt)
                };
                return InlineOrRef::Inline(
                    FlatSchema {
                        description: variant.description().to_owned(),
                        ty: Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: vec![tag.to_owned(), content.to_owned()],
                            properties: BTreeMap::from([
                                (
                                    tag.to_owned(),
                                    InlineOrRef::Inline(
                                        FlatSchema {
                                            description: variant.description().to_owned(),
                                            ty: Type::String {
                                                values: all_tags.map(Vec::from),
                                            },
                                        }
                                        .into(),
                                    ),
                                ),
                                (content.to_owned(), data),
                            ]),
                        },
                    }
                    .into(),
                );
            }
            crate::Representation::Internal { tag } => {
                let tag_field = crate::Field {
                    name: tag.to_owned(),
                    serde_name: tag.to_owned(),
                    description: "tag".to_owned(),
                    type_ref: crate::TypeReference::new("std::string::String".into(), vec![]),
                    required: true,
                    flattened: false,
                    transform_callback: String::new(),
                    transform_callback_fn: None,
                };

                match &mut strukt.fields {
                    reflectapi_schema::Fields::Named(fields) => fields.push(tag_field),
                    reflectapi_schema::Fields::None => {
                        strukt.fields = reflectapi_schema::Fields::Named(vec![tag_field])
                    }
                    reflectapi_schema::Fields::Unnamed(fields) if fields.len() == 1 => {
                        let field = fields.pop().unwrap();
                        let s = self.convert_type_ref(schema, kind, &field.type_ref);
                        return self.internally_tag(
                            all_tags.expect("expected some for internally tagged enum"),
                            tag,
                            &s,
                        );
                    }
                    reflectapi_schema::Fields::Unnamed(_) => {
                        panic!(
                            "internal repr with unnamed fields is not allowed by serde: {}",
                            adt.name()
                        )
                    }
                }
            }
            crate::Representation::None => {
                if variant.fields.len() == 1 && !variant.fields[0].is_named() {
                    return self.convert_type_ref(schema, kind, variant.fields[0].type_ref());
                }

                if variant.fields.is_empty() {
                    // ```rust
                    // #[derive(Serialize)]
                    // #[serde(untagged)]
                    // enum {
                    //    A,            // null
                    //    B {}          // {}
                    //    C()           // []
                    // }
                    // ```

                    let ty = match variant.fields {
                        reflectapi_schema::Fields::None => Type::Null,
                        reflectapi_schema::Fields::Named(_) => Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: vec![],
                            properties: BTreeMap::new(),
                        },
                        reflectapi_schema::Fields::Unnamed(_) => Type::Tuple {
                            prefix_items: vec![],
                        },
                    };

                    return InlineOrRef::Inline(
                        FlatSchema {
                            description: variant.description().to_owned(),
                            ty,
                        }
                        .into(),
                    );
                }
            }
        }

        self.struct_to_schema(schema, kind, &type_ref, &strukt)
    }

    fn internally_tag(
        &self,
        all_tags: &[String],
        tag: &str,
        schema: &InlineOrRef<Schema>,
    ) -> InlineOrRef<Schema> {
        match self.resolve_schema_ref(schema) {
            Schema::AllOf { subschemas } => {
                let mut subschemas = subschemas.clone();
                subschemas.push(InlineOrRef::Inline(FlatSchema {
                    description: "tag object".to_owned(),
                    ty: Type::Object{
                        title: "tag".to_owned(),
                        required: vec![tag.to_owned()],
                        properties: BTreeMap::from([(tag.to_owned(), InlineOrRef::Inline(Schema::Flat(FlatSchema {
                            description: "tag".to_owned(),
                            ty: Type::String {
                                values: Some(all_tags.to_vec()),
                            },
                        })))]),
                    },
                }.into()));
                InlineOrRef::Inline(Schema::AllOf { subschemas })
            }
            Schema::OneOf { subschemas } => return InlineOrRef::Inline(Schema::OneOf {
                subschemas: subschemas.iter().map(|subschema| self.internally_tag(all_tags, tag, subschema)).collect()
            }),
            Schema::Flat(schema) => match &schema.ty {
                Type::Boolean
                | Type::Integer
                | Type::Number
                | Type::Null
                | Type::String { .. }
                | Type::Array { .. }
                | Type::Tuple { .. } => panic!(
                    "newtype variant containing `{:?}` will panic on serialization, either mark this variant as `untagged` or use a different repr",
                    schema.ty,
                ),
                Type::Map { .. } => todo!("map within newtype variant"),
                Type::Object {
                    title,
                    required,
                    properties,
                } => {
                    return InlineOrRef::Inline(Schema::Flat(FlatSchema {
                        description: schema.description.to_owned(),
                        ty: Type::Object {
                            title: title.to_owned(),
                            required: required
                                .iter()
                                .cloned()
                                .chain(std::iter::once(tag.to_owned()))
                                .collect(),
                            properties: properties
                                .iter()
                                .map(|(k, v)| (k.to_owned(), v.clone()))
                                .chain(std::iter::once((
                                    tag.to_owned(),
                                    InlineOrRef::Inline(Schema::Flat(FlatSchema {
                                        description: "tag".to_owned(),
                                        ty: Type::String {
                                            values: None
                                        },
                                    })),
                                )))
                                .collect(),
                        },
                    }))
                }
            },
        }
    }

    fn struct_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        type_ref: &crate::TypeReference,
        strukt: &crate::Struct,
    ) -> InlineOrRef<Schema> {
        assert!(
            strukt.parameters.is_empty(),
            "expect generic struct to be instantiated"
        );

        if strukt.transparent() {
            assert_eq!(strukt.fields().len(), 1);
            return self.convert_type_ref(schema, kind, strukt.fields[0].type_ref());
        }

        let (flattened_fields, fields) = strukt.fields().partition::<Vec<_>, _>(|f| f.flattened);

        let ty = if fields.iter().all(|f| f.is_named()) {
            Type::Object {
                title: fmt_type_ref(type_ref),
                required: fields
                    .iter()
                    .filter(|f| f.required)
                    .map(|f| f.serde_name().to_owned())
                    .collect(),
                properties: fields
                    .iter()
                    .map(|field| {
                        (field.serde_name().to_owned(), {
                            self.convert_type_ref(schema, kind, field.type_ref())
                        })
                    })
                    .collect(),
            }
        } else {
            assert!(
                flattened_fields.is_empty(),
                "tuple cannot have flattened fields"
            );
            Type::Tuple {
                prefix_items: fields
                    .iter()
                    .map(|field| self.convert_type_ref(schema, kind, field.type_ref()))
                    .collect(),
            }
        };

        let s = FlatSchema {
            description: strukt.description().to_owned(),
            ty,
        }
        .into();

        let s = if flattened_fields.is_empty() {
            s
        } else {
            let subschemas = flattened_fields
                .into_iter()
                .map(|field| self.convert_type_ref(schema, kind, field.type_ref()))
                .chain(std::iter::once(InlineOrRef::Inline(s)))
                .collect::<Vec<_>>();

            if subschemas.len() == 1 {
                return subschemas.into_iter().next().unwrap();
            } else {
                Schema::AllOf { subschemas }
            }
        };

        InlineOrRef::Inline(s)
    }
}

fn normalize(name: impl AsRef<str>) -> String {
    name.as_ref().replace("::", ".")
}

fn fmt_type_ref(r: &crate::TypeReference) -> String {
    if r.arguments.is_empty() {
        normalize(&r.name)
    } else {
        format!(
            "{}<{}>",
            normalize(&r.name),
            r.arguments
                .iter()
                .map(fmt_type_ref)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Kind {
    Input,
    Output,
}
