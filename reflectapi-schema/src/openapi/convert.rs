//! Convert `reflectapi.json` to a approximately equivalent `openapi.json` file.

// `reflectapi` constructs within this file are always prefixed with `crate::` to avoid confusion
// with `openapi` constructs of the same name.
//
// Since `reflectapi` has a richer type system than `openapi`, the conversion is not perfect.
// Generics are effectively monomorphized so each instantiation of a generic type is treated as a
// separate type in openapi.

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
                    "application/json".to_string(),
                    MediaType {
                        schema: self.convert_type_ref(schema, Kind::Input, ty),
                    },
                )]),
                required: true,
            }),
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
    ) -> Ref<Schema> {
        let name = mangle(ty_ref);
        let schema_ref = Ref::new(format!("#/components/schemas/{}", mangle(ty_ref)));
        if let Some(_schema) = self.components.schemas.get(&name) {
            return schema_ref;
        }

        let reflect_ty = match kind {
            Kind::Input => schema
                .input_types
                .get_type(&ty_ref.name)
                .unwrap_or_else(|| panic!("input type not found: `{ty_ref:?}`")),
            Kind::Output => &schema
                .output_types
                .get_type(&ty_ref.name)
                .unwrap_or_else(|| panic!("output type not found: `{ty_ref:?}`")),
        };

        let schema = match reflect_ty {
            crate::Type::Struct(strukt) => CompositeSchema::Schema(self.struct_to_schema(
                schema,
                kind,
                &strukt.clone().instantiate(&ty_ref.arguments),
            )),
            crate::Type::Enum(adt) => {
                self.enum_to_schema(schema, kind, &adt.clone().instantiate(&ty_ref.arguments))
            }
            crate::Type::Primitive(prim) => {
                let ty = match prim.name() {
                    "f32" | "f64" => Type::Number,
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32"
                    | "u64" | "u128" | "usize" => Type::Integer,
                    "bool" => Type::Boolean,
                    "char" | "std::string::String" => Type::String,
                    "()" => Type::Null,
                    "std::vec::Vec" | "std::array::Array" => Type::Array {
                        items: Box::new(InlineOrRef::Ref(self.convert_type_ref(
                            schema,
                            kind,
                            ty_ref.arguments().next().unwrap(),
                        ))),
                    },
                    _ => todo!("primitive: {}", prim.name()),
                };
                CompositeSchema::Schema(Schema {
                    description: reflect_ty.description().to_owned(),
                    ty,
                })
            }
        };

        assert!(self.components.schemas.insert(name, schema).is_none());
        schema_ref
    }

    fn enum_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        adt: &crate::Enum,
    ) -> CompositeSchema {
        assert!(adt.parameters.is_empty(), "expect enum to be instantiated");

        let subschemas = adt
            .variants()
            .map(|variant| self.variant_to_schema(schema, kind, variant))
            .map(InlineOrRef::Inline)
            .collect();
        CompositeSchema::OneOf { subschemas }
    }

    fn variant_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        variant: &crate::Variant,
    ) -> Schema {
        let strukt = crate::Struct {
            name: variant.name().to_owned(),
            serde_name: variant.serde_name.to_owned(),
            description: variant.description().to_owned(),
            parameters: vec![],
            fields: variant.fields.clone(),
            transparent: false,
        };

        self.struct_to_schema(schema, kind, &strukt)
    }

    fn struct_to_schema(
        &mut self,
        schema: &crate::Schema,
        kind: Kind,
        strukt: &crate::Struct,
    ) -> Schema {
        assert!(
            strukt.parameters.is_empty(),
            "expect generic strukt to be instantiated"
        );
        let ty = Type::Object {
            properties: BTreeMap::from_iter(strukt.fields().map(|field| {
                (field.serde_name().to_owned(), {
                    InlineOrRef::Ref(self.convert_type_ref(schema, kind, field.type_ref()))
                })
            })),
        };

        Schema {
            description: strukt.description().to_owned(),
            ty,
        }
    }
}

/// Format type reference to string that only includes [a-zA-Z0-9-_]
fn mangle(ty: &crate::TypeReference) -> String {
    fn normalize(name: &str) -> String {
        if name == "()" {
            return "Unit".into();
        }

        name.replace("::", ".")
    }

    let mut s = normalize(ty.name());
    // need some better unambiguous encoding for generics, we only have `[a-zA-Z0-9-_]` to work with I think
    for arg in ty.arguments() {
        s.push_str("__");
        s.push_str(&mangle(arg));
        s.push_str("__");
    }
    s
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Kind {
    Input,
    Output,
}
