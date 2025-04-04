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
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::sync::OnceLock;
use InlineOrRef::Inline;

use reflectapi_schema::{Instantiate, Substitute};
use serde::Serialize;

#[derive(Debug, Default)]
pub struct Config {
    /// This list of tags is used to control the ordering in documentation.
    /// Tags that appear in a functions tags list but not in this list may be ordered arbitrarily.
    tags: Vec<Tag>,
    /// Only include handlers with these tags (empty means include all).
    include_tags: BTreeSet<String>,
    /// Exclude handlers with these tags (empty means exclude none).
    exclude_tags: BTreeSet<String>,
}

impl Config {
    pub fn tags(&mut self, tags: Vec<Tag>) -> &mut Self {
        self.tags = tags;
        self
    }

    pub fn include_tags(&mut self, tags: BTreeSet<String>) -> &mut Self {
        self.include_tags = tags;
        self
    }

    pub fn exclude_tags(&mut self, tags: BTreeSet<String>) -> &mut Self {
        self.exclude_tags = tags;
        self
    }
}

pub fn generate(schema: &crate::Schema, config: &Config) -> anyhow::Result<String> {
    let spec = generate_spec(schema, config);
    Ok(serde_json::to_string_pretty(&spec)?)
}

pub fn generate_spec(schema: &crate::Schema, config: &Config) -> Spec {
    Converter {
        config,
        components: Default::default(),
    }
    .convert(schema)
}

impl From<&crate::Schema> for Spec {
    fn from(schema: &crate::Schema) -> Self {
        Converter {
            config: &Config::default(),
            components: Default::default(),
        }
        .convert(schema)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spec {
    pub openapi: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    pub info: Info,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub paths: BTreeMap<String, PathItem>,
    pub components: Components,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
        }
    }

    pub fn with_description(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
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
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    tags: BTreeSet<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_body: Option<RequestBody>,
    responses: BTreeMap<String, Response>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "is_false")]
    deprecated: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    /// full-date notation as defined by RFC 3339, section 5.6, for example, 2017-07-21
    Date,
    /// date-time notation as defined by RFC 3339, section 5.6, for example, 2017-07-21T17:32:28Z
    DateTime,
    Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Boolean,
    Integer,
    Number,
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<StringFormat>,
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
        #[serde(skip_serializing_if = "BTreeSet::is_empty")]
        required: BTreeSet<String>,
        properties: BTreeMap<String, Property>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    // This `description` overrides the description of the type.
    description: String,
    deprecated: bool,
    schema: InlineOrRef<Schema>,
}

impl serde::Serialize for Property {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Debug, Clone, PartialEq, Serialize)]
        pub struct P<'a> {
            #[serde(skip_serializing_if = "str::is_empty")]
            description: &'a str,
            #[serde(skip_serializing_if = "is_false")]
            deprecated: bool,
            #[serde(flatten)]
            schema: &'a InlineOrRef<Schema>,
        }

        // Remove the description of the type if the field has a description.
        // serde_json will serialize both and generate invalid JSON.
        let schema = match &self.schema {
            Inline(schema) if !self.description.is_empty() => match schema {
                Schema::Const { value, description } if !description.is_empty() => {
                    Cow::Owned(Inline(Schema::Const {
                        value: value.clone(),
                        description: String::new(),
                    }))
                }
                Schema::Flat(flat) if !flat.description.is_empty() => {
                    Cow::Owned(Inline(Schema::Flat(FlatSchema {
                        description: String::new(),
                        ty: flat.ty.clone(),
                    })))
                }
                _ => Cow::Borrowed(&self.schema),
            },
            _ => Cow::Borrowed(&self.schema),
        };

        P {
            description: &self.description,
            deprecated: self.deprecated,
            schema: &schema,
        }
        .serialize(serializer)
    }
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
    Const {
        #[serde(rename = "const")]
        value: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        description: String,
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
            Schema::Const { value, description } => f
                .debug_struct("Const")
                .field("value", value)
                .field("description", description)
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
                title: Default::default(),
                required: Default::default(),
                properties: Default::default(),
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

#[derive(Debug)]
struct Converter<'a> {
    components: Components,
    config: &'a Config,
}

impl Converter<'_> {
    fn convert(mut self, schema: &crate::Schema) -> Spec {
        Spec {
            openapi: "3.1.0".into(),
            tags: self.config.tags.clone(),
            info: Info {
                title: schema.name.clone(),
                description: schema.description.clone(),
                version: "1.0.0".into(),
            },
            paths: schema
                .functions
                .iter()
                .filter(|f| {
                    self.config.include_tags.is_empty()
                        || f.tags
                            .iter()
                            .any(|tag| self.config.include_tags.contains(tag))
                })
                .filter(|f| {
                    self.config.exclude_tags.is_empty()
                        || f.tags
                            .iter()
                            .all(|tag| !self.config.exclude_tags.contains(tag))
                })
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
        let ok_response = Response {
            description: "200 OK".to_owned(),
            content: BTreeMap::from([(
                "application/json".to_owned(),
                MediaType {
                    schema: f.output_type().map_or_else(
                        || Inline(Schema::Flat(FlatSchema::empty_object())),
                        |ty| self.convert_type_ref(schema, Kind::Output, ty),
                    ),
                },
            )]),
        };

        let mut responses = BTreeMap::new();
        responses.insert("200".to_owned(), ok_response);
        if let Some(err) = f.error_type() {
            let err_response = Response {
                description: "Error cases".to_owned(),
                content: BTreeMap::from([(
                    "application/json".to_owned(),
                    MediaType {
                        schema: self.convert_type_ref(schema, Kind::Output, err),
                    },
                )]),
            };
            responses.insert("default".to_owned(), err_response);
        }

        let operation = Operation {
            operation_id: f.name.clone(),
            tags: f.tags.clone(),
            description: f.description.clone(),
            deprecated: f.deprecated(),
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
            responses,
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
                .map(|(name, property)| {
                    let required = required.contains(&name);
                    Parameter {
                        name,
                        location: In::Header,
                        required,
                        schema: property.schema,
                    }
                })
                .collect(),
            _ => unreachable!("header type is a struct"),
        }
    }

    fn convert_field(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        field: &crate::Field,
    ) -> Property {
        Property {
            description: field.description().to_owned(),
            deprecated: field.deprecated(),
            schema: self.convert_type_ref(schema, kind, field.type_ref()),
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
                // HACK: string type is often used as a fallback type.
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
                            Inline(Schema::Flat(FlatSchema {
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
                        Inline(schema) => schema,
                        InlineOrRef::Ref(r) => {
                            self.components.schemas.remove(&name);
                            return InlineOrRef::Ref(r);
                        }
                    }
                }
            }
            crate::Type::Primitive(prim) => {
                assert_eq!(
                    prim.parameters.len(),
                    ty_ref.arguments.len(),
                    "primitive type with wrong number of arguments: {ty_ref:?}, expected {}, got {}",
                    prim.parameters.len(),
                    ty_ref.arguments.len()
                );
                let ty = match prim.name() {
                    "f32" | "f64" => Type::Number,
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32"
                    | "u64" | "u128" | "usize" => Type::Integer,
                    "bool" => Type::Boolean,
                    "serde_json::Value" => Type::Object {
                        // Not sure if there is a better way to represent this
                        title: "serde_json::Value".to_owned(),
                        required: Default::default(),
                        properties: Default::default(),
                    },
                    "uuid::Uuid" => Type::String {
                        format: Some(StringFormat::Uuid),
                    },
                    "chrono::NaiveDate" => Type::String {
                        format: Some(StringFormat::Date),
                    },
                    "chrono::NaiveDateTime" | "chrono::DateTime" => Type::String {
                        format: Some(StringFormat::DateTime),
                    },
                    "char" | "std::string::String" => Type::String { format: None },
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
            Inline(schema)
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

        let subschemas = adt
            .variants()
            .map(|variant| self.variant_to_schema(schema, kind, type_ref, adt, variant))
            .collect::<Vec<_>>();

        let schema = match adt.representation() {
            crate::Representation::Internal { tag: _ } => Schema::OneOf { subschemas },
            crate::Representation::External
            | crate::Representation::Adjacent { .. }
            | crate::Representation::None => Schema::OneOf { subschemas },
        };

        Inline(schema)
    }

    fn variant_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        type_ref: &crate::TypeReference,
        adt: &crate::Enum,
        variant: &crate::Variant,
    ) -> InlineOrRef<Schema> {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        fn mk_tag(tag: impl Into<String>) -> InlineOrRef<Schema> {
            Inline(Schema::Const {
                value: tag.into(),
                description: String::new(),
            })
        }

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
                            required: Default::default(),
                            properties: BTreeMap::from_iter([(
                                variant.serde_name().to_owned(),
                                Property {
                                    description: variant.description().to_owned(),
                                    deprecated: false,
                                    schema: Inline(Schema::empty_object()),
                                },
                            )]),
                        },
                        reflectapi_schema::Fields::Unnamed(_) => Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: Default::default(),
                            properties: BTreeMap::from_iter([(
                                variant.serde_name().to_owned(),
                                Property {
                                    description: variant.description().to_owned(),
                                    deprecated: false,
                                    schema: Inline(Schema::empty_tuple()),
                                },
                            )]),
                        },
                        reflectapi_schema::Fields::None => {
                            return Inline(Schema::Const {
                                description: variant.description().to_owned(),
                                value: variant.serde_name().to_owned(),
                            })
                        }
                    };

                    return Inline(
                        FlatSchema {
                            description: variant.description().to_owned(),
                            ty,
                        }
                        .into(),
                    );
                }

                let property_schema = if variant.fields.len() == 1 && !variant.fields[0].is_named()
                {
                    self.convert_type_ref(schema, kind, variant.fields[0].type_ref())
                } else {
                    self.struct_to_schema(schema, kind, &type_ref, &strukt)
                };

                return Inline(
                    FlatSchema {
                        description: variant.description().to_owned(),
                        ty: Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: [variant.serde_name().to_owned()].into(),
                            properties: BTreeMap::from([(
                                variant.serde_name().to_owned(),
                                Property {
                                    deprecated: false,
                                    description: String::new(),
                                    schema: property_schema,
                                },
                            )]),
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
                return Inline(
                    FlatSchema {
                        description: variant.description().to_owned(),
                        ty: Type::Object {
                            title: fmt_type_ref(&type_ref),
                            required: [tag.to_owned(), content.to_owned()].into(),
                            properties: BTreeMap::from([
                                (
                                    tag.to_owned(),
                                    Property {
                                        description: String::new(),
                                        deprecated: false,
                                        schema: mk_tag(&variant.name),
                                    },
                                ),
                                (
                                    content.to_owned(),
                                    Property {
                                        description: String::new(),
                                        deprecated: false,
                                        schema: data,
                                    },
                                ),
                            ]),
                        },
                    }
                    .into(),
                );
            }
            crate::Representation::Internal { tag } => {
                let s = match &mut strukt.fields {
                    reflectapi_schema::Fields::Named(_) => {
                        self.struct_to_schema(schema, kind, &type_ref, &strukt)
                    }
                    reflectapi_schema::Fields::None => Inline(Schema::Flat(FlatSchema {
                        description: String::new(),
                        ty: Type::Object {
                            title: String::new(),
                            required: [tag.to_owned()].into(),
                            properties: [(
                                tag.to_owned(),
                                Property {
                                    description: String::new(),
                                    deprecated: false,
                                    schema: mk_tag(&variant.name),
                                },
                            )]
                            .into(),
                        },
                    })),
                    reflectapi_schema::Fields::Unnamed(fields) if fields.len() == 1 => {
                        let field = fields.pop().unwrap();
                        self.convert_type_ref(schema, kind, &field.type_ref)
                    }
                    reflectapi_schema::Fields::Unnamed(_) => {
                        panic!(
                            "internal repr with unnamed fields is not allowed by serde: {}",
                            adt.name()
                        )
                    }
                };

                return self
                    .internally_tag(tag, mk_tag(&variant.name), &s)
                    .unwrap_or_else(|err| panic!("{}: {err}", adt.name()));
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
                            required: Default::default(),
                            properties: BTreeMap::new(),
                        },
                        reflectapi_schema::Fields::Unnamed(_) => Type::Tuple {
                            prefix_items: vec![],
                        },
                    };

                    return Inline(
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
        tag_name: &str,
        tag_schema: InlineOrRef<Schema>,
        schema: &InlineOrRef<Schema>,
    ) -> Result<InlineOrRef<Schema>, InvalidInternalTagError> {
        match self.resolve_schema_ref(schema) {
            Schema::AllOf { subschemas } => {
                let mut subschemas = subschemas.clone();
                subschemas.push(Inline(
                    FlatSchema {
                        description: "tag object".to_owned(),
                        ty: Type::Object {
                            title: "tag".to_owned(),
                            required: [tag_name.to_owned()].into(),
                            properties: BTreeMap::from([(
                                tag_name.to_owned(),
                                Property {
                                    description: String::new(),
                                    deprecated: false,
                                    schema: tag_schema,
                                },
                            )]),
                        },
                    }
                    .into(),
                ));
                Ok(Inline(Schema::AllOf { subschemas }))
            }
            Schema::OneOf { subschemas } => Ok(Inline(Schema::OneOf {
                subschemas: subschemas
                    .iter()
                    .map(|subschema| self.internally_tag(tag_name, tag_schema.clone(), subschema))
                    .collect::<Result<_, _>>()?,
            })),
            Schema::Const { .. } => unreachable!("internally tagged const?"),
            Schema::Flat(schema) => match &schema.ty {
                Type::Boolean
                | Type::Integer
                | Type::Number
                | Type::Null
                | Type::String { .. }
                | Type::Array { .. }
                | Type::Tuple { .. } => Err(InvalidInternalTagError(schema.ty.clone())),
                Type::Map { .. } => todo!("map within newtype variant"),
                Type::Object {
                    title,
                    required,
                    properties,
                } => {
                    return Ok(Inline(Schema::Flat(FlatSchema {
                        description: schema.description.to_owned(),
                        ty: Type::Object {
                            title: title.to_owned(),
                            required: required
                                .iter()
                                .cloned()
                                .chain(std::iter::once(tag_name.to_owned()))
                                .collect(),
                            properties: properties
                                .iter()
                                .map(|(k, v)| (k.to_owned(), v.clone()))
                                .chain(std::iter::once((
                                    tag_name.to_owned(),
                                    Property {
                                        description: String::new(),
                                        deprecated: false,
                                        schema: tag_schema,
                                    },
                                )))
                                .collect(),
                        },
                    })))
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
                        (
                            field.serde_name().to_owned(),
                            self.convert_field(schema, kind, field),
                        )
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
                .chain(std::iter::once(Inline(s)))
                .collect::<Vec<_>>();

            if subschemas.len() == 1 {
                return subschemas.into_iter().next().unwrap();
            } else {
                Schema::AllOf { subschemas }
            }
        };

        Inline(s)
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

struct InvalidInternalTagError(Type);

impl fmt::Debug for InvalidInternalTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for InvalidInternalTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "newtype variant containing `{:?}` will panic on serialization, either mark this variant as `untagged` or use a different repr", self.0)
    }
}

impl std::error::Error for InvalidInternalTagError {}
