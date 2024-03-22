use std::collections::HashMap;

use anyhow::Context;

use askama::Template; // bring trait in scope

#[derive(Template)]
#[template(
    source = "// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflect-cli
//
// Schema name: {{ name }}
// {{ description }}

",
    ext = "txt"
)]
struct FileTemplate {
    name: String,
    description: String,
}

#[derive(Template)]
#[template(
    source = "{{ self.render_start() }}
{% for type in types.iter() %}
{{ type }}
{% endfor -%}
{% for module in self.submodules_sorted() %}
{{ module }}
{% endfor -%}
{{ self.render_end() }}",
    ext = "txt"
)]
struct Module {
    name: String,
    types: Vec<String>,
    submodules: HashMap<String, Module>,
}

impl Module {
    fn submodules_sorted(&self) -> Vec<&Module> {
        let mut submodules = self.submodules.values().collect::<Vec<_>>();
        submodules.sort_by(|a, b| a.name.cmp(&b.name));
        submodules
    }

    fn is_empty(&self) -> bool {
        self.types.is_empty() && self.submodules.iter().all(|(_, m)| m.is_empty())
    }

    fn render_start(&self) -> String {
        if self.name.is_empty() || self.is_empty() {
            "".into()
        } else {
            format!("export namespace {} {{", self.name)
        }
    }

    fn render_end(&self) -> String {
        if self.name.is_empty() || self.is_empty() {
            "".into()
        } else {
            "}".into()
        }
    }
}

#[derive(Template)]
#[template(
    source = "{{ description }}export interface {{ name }} {
    {% for field in fields.iter() -%}
    {{ field }};
    {% endfor -%}
}
",
    ext = "txt"
)]
struct StructTemplate {
    name: String,
    description: String,
    fields: Vec<FieldTemplate>,
}

#[derive(Template)]
#[template(source = "{{ description }}{{ name }}: {{ type_ }}", ext = "txt")]
struct FieldTemplate {
    name: String,
    description: String,
    type_: String,
}

#[derive(Template)]
#[template(
    source = "{{ description }}export type {{ name }} =
    {% for variant in variants.iter() -%}
    {{ variant }}
    {% endfor %};
",
    ext = "txt"
)]
struct EnumTemplate {
    name: String,
    description: String,
    variants: Vec<VariantTemplate>,
}

#[derive(Template)]
#[template(source = "{{ description }}| {{ self.render_self()? }}", ext = "txt")]
struct VariantTemplate {
    name: String,
    description: String,
    representation: crate::Representation,
    fields: Vec<VariantFieldTemplate>,
}

impl VariantTemplate {
    fn fields_brakets(&self) -> (String, String) {
        if self.fields.is_empty() {
            ("".into(), "".into())
        } else if self.fields.iter().all(|f| f.is_unnamed()) {
            if self.fields.len() == 1 {
                ("".into(), "".into())
            } else {
                ("[".into(), "]".into())
            }
        } else {
            ("{".into(), "}".into())
        }
    }

    fn render_self(&self) -> anyhow::Result<String> {
        if self.fields.is_empty() {
            return Ok(format!("\"{}\"", self.name));
        }
        let r = match &self.representation {
            crate::Representation::External => {
                format!("{{ {}: {} }}", self.name, self.render_fields(None)?)
            }
            crate::Representation::Internal { tag } => self.render_fields(Some(tag))?,
            crate::Representation::Adjacent { tag, content } => {
                format!(
                    "{{ {}: {}, {}: {} }}",
                    tag,
                    self.name,
                    content,
                    self.render_fields(None)?
                )
            }
            crate::Representation::None => self.render_fields(None)?,
        };
        Ok(r)
    }

    fn render_fields(&self, inner_tag: Option<&str>) -> anyhow::Result<String> {
        let brackets = self.fields_brakets();
        let mut rendered_fields = Vec::new();
        if let Some(inner_tag) = inner_tag {
            rendered_fields.push(format!("{}: {}", inner_tag, self.name));
        }
        for field in self.fields.iter() {
            rendered_fields.push(field.render()?);
        }
        Ok(format!(
            "{}{}{}",
            brackets.0,
            rendered_fields.join(",\n"),
            brackets.1
        ))
    }
}

#[derive(Template)]
#[template(
    source = "{{ description }}{% if !self.is_unnamed() %}{{ name }}: {{ type_ }}{% else %}{{ type_ }}{% endif  %}",
    ext = "txt"
)]
struct VariantFieldTemplate {
    name: String,
    description: String,
    type_: String,
}

impl VariantFieldTemplate {
    fn is_unnamed(&self) -> bool {
        self.name.parse::<u64>().is_ok()
    }
}

#[derive(Template)]
#[template(
    source = "{{ description }}export type {{ name }} = {{ type_ }};
",
    ext = "txt"
)]
struct PrimitiveTemplate {
    name: String,
    description: String,
    type_: String,
}

pub fn generate(mut schema: crate::Schema) -> anyhow::Result<String> {
    let file_template = FileTemplate {
        name: schema.name.clone(),
        description: schema.description.clone(),
    };
    let mut generated_code = vec![];
    generated_code.push(
        file_template
            .render()
            .context("Failed to render template")?,
    );

    let implemented_types = implemented_types();

    let mut rendered_types = HashMap::new();
    for original_type_name in schema.consolidate_types() {
        let type_def = schema.get_type(&original_type_name).context(format!(
            "internal error: failed to get consolidated type definition for type: {}",
            original_type_name
        ))?;
        if implemented_types.contains_key(&original_type_name) {
            continue;
        }
        if type_def.fallback().is_some() {
            continue;
        }
        rendered_types.insert(
            original_type_name,
            render_type(type_def, &schema, &implemented_types)?,
        );
    }

    let module = modules_from_types(schema.consolidate_types(), rendered_types);
    generated_code.push(module.render().context("Failed to render template")?);

    let generated_code = generated_code.join("\n");
    Ok(generated_code)
}

fn render_type(
    type_def: &crate::Type,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Result<String, anyhow::Error> {
    let type_name = type_to_ts_name(&type_def);

    Ok(match type_def {
        crate::Type::Struct(struct_def) => {
            let struct_template = StructTemplate {
                name: type_name,
                description: doc_to_ts_comments(&struct_def.description, 0),
                fields: struct_def
                    .fields
                    .iter()
                    .map(|field| FieldTemplate {
                        name: field.name.clone(),
                        description: doc_to_ts_comments(&field.description, 4),
                        type_: type_ref_to_ts_ref(&field.type_ref, schema, implemented_types),
                    })
                    .collect::<Vec<_>>(),
            };
            struct_template
                .render()
                .context("Failed to render template")?
        }
        crate::Type::Enum(enum_def) => {
            let enum_template = EnumTemplate {
                name: type_name,
                description: doc_to_ts_comments(&enum_def.description, 0),
                variants: enum_def
                    .variants
                    .iter()
                    .map(|variant| VariantTemplate {
                        name: variant.name.clone(),
                        description: doc_to_ts_comments(&variant.description, 4),
                        representation: enum_def.representation.clone(),
                        fields: variant
                            .fields
                            .iter()
                            .map(|field| VariantFieldTemplate {
                                name: field.name.clone(),
                                description: doc_to_ts_comments(&field.description, 8),
                                type_: type_ref_to_ts_ref(
                                    &field.type_ref,
                                    schema,
                                    implemented_types,
                                ),
                            })
                            .collect::<Vec<_>>(),
                    })
                    .collect::<Vec<_>>(),
            };
            enum_template
                .render()
                .context("Failed to render template")?
        }
        crate::Type::Primitive(type_def) => {
            eprintln!(
                "warning: {} type is not implemented for Typescript",
                type_def.name
            );
            let primitive_template = PrimitiveTemplate {
                name: type_name,
                description: doc_to_ts_comments(&type_def.description, 0),
                type_: format!(
                    "any /* fallback to any for unimplemented type: {} */",
                    type_def.name
                ),
            };
            primitive_template
                .render()
                .context("Failed to render template")?
        }
    })
}

fn type_ref_to_ts_ref(
    type_ref: &crate::TypeReference,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> String {
    if let Some(resolved_type) = resolve_type_ref(type_ref, schema, implemented_types) {
        return resolved_type;
    }

    let type_name_parts = type_ref.name.split("::").collect::<Vec<_>>();
    let n = if type_name_parts.len() > 2 && type_name_parts[0] == "std" {
        format!("std.{}", type_name_parts[type_name_parts.len() - 1])
    } else {
        type_name_parts.join(".")
    };
    let p = type_ref_params_to_ts_ref(&type_ref.parameters, schema, implemented_types);
    format!("{}{}", n, p)
}

fn type_ref_params_to_ts_ref(
    type_params: &Vec<crate::TypeReference>,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> String {
    let p = type_params
        .iter()
        .map(|type_ref| type_ref_to_ts_ref(type_ref, schema, implemented_types))
        .collect::<Vec<_>>()
        .join(", ");
    if p.is_empty() {
        p
    } else {
        format!("<{}>", p)
    }
}

fn type_to_ts_name(type_: &crate::Type) -> String {
    let n = type_
        .name()
        .split("::")
        .last()
        .unwrap_or_default()
        .to_string();
    let p = type_params_to_ts_name(type_.parameters());
    format!("{}{}", n, p)
}

fn type_params_to_ts_name(type_params: std::slice::Iter<'_, crate::TypeParameter>) -> String {
    let p = type_params
        .map(|type_param| type_param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    if p.is_empty() {
        p
    } else {
        format!("<{}>", p)
    }
}

fn doc_to_ts_comments(doc: &str, offset: u8) -> String {
    if doc.is_empty() {
        return "".into();
    }

    let offset = " ".repeat(offset as usize);
    let doc = doc.split("\n").collect::<Vec<_>>();
    let sp = if doc.iter().all(|i| i.starts_with(" ")) {
        ""
    } else {
        " "
    };
    let doc = doc
        .iter()
        .map(|line| format!("///{}{}", sp, line))
        .collect::<Vec<_>>()
        .join(format!("\n{}", offset).as_str());
    format!("{}\n{}", doc, offset)
}

fn modules_from_types(types: Vec<String>, mut rendered_types: HashMap<String, String>) -> Module {
    let mut root_module = Module {
        name: "".into(),
        types: vec![],
        submodules: HashMap::new(),
    };

    for original_type_name in types {
        let mut module = &mut root_module;
        let parts = original_type_name.split("::").collect::<Vec<_>>();
        let type_name = if parts.len() > 2 && parts[0] == "std" {
            format!("std::{}", parts[parts.len() - 1])
        } else {
            parts.join("::")
        };
        let mut parts = type_name.split("::").collect::<Vec<_>>();
        parts.pop().unwrap();
        for part in parts {
            module = module.submodules.entry(part.into()).or_insert(Module {
                name: part.into(),
                types: vec![],
                submodules: HashMap::new(),
            });
        }
        if let Some(rendered_type) = rendered_types.remove(&original_type_name) {
            module.types.push(rendered_type);
        }
    }

    root_module
}

fn implemented_types() -> HashMap<String, String> {
    let mut implemented_types = HashMap::new();
    implemented_types.insert("u8".into(), "number /* u8 */".into());
    implemented_types.insert("u16".into(), "number /* u16 */".into());
    implemented_types.insert("u32".into(), "number /* u32 */".into());
    implemented_types.insert("u64".into(), "number /* u64 */".into());
    implemented_types.insert("u128".into(), "number /* u128 */".into());
    implemented_types.insert("i8".into(), "number /* i8 */".into());
    implemented_types.insert("i16".into(), "number /* i16 */".into());
    implemented_types.insert("i32".into(), "number /* i32 */".into());
    implemented_types.insert("i64".into(), "number /* i64 */".into());
    implemented_types.insert("i128".into(), "number /* i128 */".into());

    implemented_types.insert("f32".into(), "number /* f32 */".into());
    implemented_types.insert("f64".into(), "number /* f64 */".into());

    implemented_types.insert("bool".into(), "boolean".into());
    implemented_types.insert("char".into(), "string".into());
    implemented_types.insert("std::string::String".into(), "string".into());
    implemented_types.insert("()".into(), "void".into());

    // warning: all generic type parameter names should match reflect defnition coming from
    // the implementation of reflect for standard types

    implemented_types.insert("std::option::Option".into(), "T | null".into());
    implemented_types.insert("reflect::Option".into(), "T | null | undefined".into());

    implemented_types.insert("std::vec::Vec".into(), "Array<T>".into());
    implemented_types.insert("std::collections::HashMap".into(), "Record<K, V>".into());

    implemented_types.insert("std::tuple::Tuple1".into(), "[T1]".into());
    implemented_types.insert("std::tuple::Tuple2".into(), "[T1, T2]".into());
    implemented_types.insert("std::tuple::Tuple3".into(), "[T1, T2, T3]".into());
    implemented_types.insert("std::tuple::Tuple4".into(), "[T1, T2, T3, T4]".into());
    implemented_types.insert("std::tuple::Tuple5".into(), "[T1, T2, T3, T4, T5]".into());
    implemented_types.insert(
        "std::tuple::Tuple6".into(),
        "[T1, T2, T3, T4, T5, T6]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple7".into(),
        "[T1, T2, T3, T4, T5, T6, T7]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple8".into(),
        "[T1, T2, T3, T4, T5, T6, T7, T8]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple9".into(),
        "[T1, T2, T3, T4, T5, T6, T7, T8, T9]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple10".into(),
        "[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple11".into(),
        "[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple12".into(),
        "[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]".into(),
    );

    // we preserve it in case the generated code might have references to unused generic parameters
    implemented_types.insert(
        "std::marker::PhantomData".into(),
        "undefined | T /* phantom data */".into(),
    );

    implemented_types
}

fn resolve_type_ref(
    type_ref: &crate::TypeReference,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Option<String> {
    let Some(mut implementation) = implemented_types.get(type_ref.name.as_str()).cloned() else {
        // TODO here we need to fallback one level if it exists
        return None;
    };

    let Some(type_def) = schema.get_type(type_ref.name()) else {
        return None;
    };

    for (type_def_param, type_ref_param) in type_def.parameters().zip(type_ref.parameters.iter()) {
        if implementation.contains(type_def_param.name.as_str()) {
            implementation = implementation.replace(
                type_def_param.name.as_str(),
                type_ref_to_ts_ref(type_ref_param, schema, implemented_types).as_str(),
            );
        }
    }

    Some(implementation)
}
