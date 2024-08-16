//! Convert `reflectapi.json` to a approximately equivalent `openapi.json` file.

// `reflectapi` constructs within this file are always prefixed with `crate::` to avoid confusion
// with `openapi` constructs of the same name.
//
// Since `reflectapi` has a richer type system than `openapi`, the conversion is not perfect.
// Generics are effectively monomorphized i.e. each instantiation of a generic type is inlined.

use super::*;

impl From<&crate::Schema> for Spec {
    fn from(schema: &crate::Schema) -> Self {
        Converter::default().convert(schema)
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
                // FIXME HACK string type is used for the tag type, but may not exist in the schema
                return Box::leak(Box::new(crate::Type::Primitive(crate::Primitive {
                    name: "std::string::String".into(),
                    description: "UTF-8 encoded string".into(),
                    parameters: vec![],
                    fallback: None,
                })));
            };

            panic!("{kind:?} type not found: {ty_ref:?}",)
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
                &strukt.clone().instantiate(&ty_ref.arguments),
            ) {
                InlineOrRef::Inline(schema) => schema,
                InlineOrRef::Ref(r) => {
                    assert!(
                        self.components.schemas.remove(&name).is_some(),
                        "remove stub"
                    );
                    return InlineOrRef::Ref(r);
                }
            },
            crate::Type::Enum(adt) => {
                if adt.name == "std::option::Option" {
                    // Special case `Option` to generate a nicer spec.
                    Schema::OneOf {
                        subschemas: vec![
                            InlineOrRef::Inline(Schema::Flat(FlatSchema {
                                description: "Null".to_owned(),
                                ty: Type::Null,
                            })),
                            self.convert_type_ref(schema, kind, ty_ref.arguments().next().unwrap()),
                        ],
                        discriminator: None,
                    }
                } else {
                    match self.enum_to_schema(
                        schema,
                        kind,
                        &adt.clone().instantiate(&ty_ref.arguments),
                    ) {
                        InlineOrRef::Inline(schema) => schema,
                        InlineOrRef::Ref(r) => {
                            assert!(
                                self.components.schemas.remove(&name).is_some(),
                                "remove stub"
                            );
                            return InlineOrRef::Ref(r);
                        }
                    }
                }
            }
            crate::Type::Primitive(prim) => {
                assert_eq!(ty_ref.arguments.len(), prim.parameters.len());
                let ty = match prim.name() {
                    "f32" | "f64" => Type::Number,
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32"
                    | "u64" | "u128" | "usize" => Type::Integer,
                    "bool" => Type::Boolean,
                    "char" | "std::string::String" => Type::String,
                    "std::marker::PhantomData" | "unit" => Type::Null,
                    "std::vec::Vec" | "std::array::Array" | "std::collections::HashSet" => {
                        Type::Array {
                            items: Box::new(self.convert_type_ref(
                                schema,
                                kind,
                                ty_ref.arguments().next().unwrap(),
                            )),
                            unique_items: prim.name() == "std::collections::HashSet",
                        }
                    }
                    // Treat as transparent wrappers? Not sure why these types are relevant to an API anyway @Andrey?
                    "std::boxed::Box" | "std::cell::Cell" | "std::cell::RefCell"
                    | "std::sync::Mutex" | "std::sync::RwLock" | "std::rc::Rc"
                    | "std::rc::Weak" | "std::sync::Arc" | "std::sync::Weak" => {
                        return self.convert_type_ref(
                            schema,
                            kind,
                            ty_ref.arguments().next().unwrap(),
                        )
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
                    _ => unimplemented!("primitive: {}", prim.name()),
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
        adt: &crate::Enum,
    ) -> InlineOrRef<Schema> {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        let subschemas = adt
            .variants()
            .map(|variant| self.variant_to_schema(schema, kind, adt, variant))
            .collect::<Vec<_>>();

        let schema = match adt.representation() {
            crate::Representation::Internal { tag } => Schema::OneOf {
                subschemas,
                discriminator: Some(Discriminator {
                    property_name: tag.to_owned(),
                }),
            },
            crate::Representation::External
            | crate::Representation::Adjacent { .. }
            | crate::Representation::None => Schema::OneOf {
                subschemas,
                discriminator: None,
            },
        };

        InlineOrRef::Inline(schema)
    }

    fn variant_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        adt: &crate::Enum,
        variant: &crate::Variant,
    ) -> InlineOrRef<Schema> {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        let mut strukt = crate::Struct {
            name: variant.name().to_owned(),
            serde_name: variant.serde_name.to_owned(),
            description: variant.description().to_owned(),
            parameters: vec![],
            fields: variant.fields.clone(),
            transparent: false,
        };

        match adt.representation() {
            crate::Representation::External => {
                return InlineOrRef::Inline(
                    FlatSchema {
                        description: "".to_owned(),
                        ty: Type::Object {
                            properties: BTreeMap::from([(
                                variant.serde_name().to_owned(),
                                self.struct_to_schema(schema, kind, &strukt),
                            )]),
                        },
                    }
                    .into(),
                );
            }
            crate::Representation::Adjacent { tag, content } => {
                return InlineOrRef::Inline(
                    FlatSchema {
                        description: "".to_owned(),
                        ty: Type::Object {
                            properties: BTreeMap::from([
                                (
                                    tag.to_owned(),
                                    InlineOrRef::Inline(
                                        FlatSchema {
                                            description: "".to_owned(),
                                            ty: Type::String,
                                        }
                                        .into(),
                                    ),
                                ),
                                (
                                    content.to_owned(),
                                    self.struct_to_schema(schema, kind, &strukt),
                                ),
                            ]),
                        },
                    }
                    .into(),
                );
            }
            crate::Representation::Internal { tag } => {
                strukt.fields.push(crate::Field {
                    name: tag.to_owned(),
                    serde_name: tag.to_owned(),
                    description: "tag".to_owned(),
                    type_ref: crate::TypeReference::new("std::string::String".into(), vec![]),
                    required: true,
                    flattened: false,
                    transform_callback: String::new(),
                    transform_callback_fn: None,
                });
            }
            crate::Representation::None => {}
        }

        self.struct_to_schema(schema, kind, &strukt)
    }

    fn struct_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
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

        let ty = if strukt.fields().all(|f| f.is_named()) {
            Type::Object {
                properties: strukt
                    .fields()
                    .map(|field| {
                        (field.serde_name().to_owned(), {
                            self.convert_type_ref(schema, kind, field.type_ref())
                        })
                    })
                    .collect(),
            }
        } else {
            Type::Tuple {
                prefix_items: strukt
                    .fields()
                    .map(|field| self.convert_type_ref(schema, kind, field.type_ref()))
                    .collect(),
            }
        };

        InlineOrRef::Inline(
            FlatSchema {
                description: strukt.description().to_owned(),
                ty,
            }
            .into(),
        )
    }
}

// OpenAPI doesn't allow `:`
fn normalize(name: &str) -> String {
    name.replace("::", ".")
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Kind {
    Input,
    Output,
}
