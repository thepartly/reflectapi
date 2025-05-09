use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    process::{Command, Stdio},
};

use super::{format_with, END_BOILERPLATE, START_BOILERPLATE};
use anyhow::Context;
use askama::Template;
use indexmap::IndexMap;
use reflectapi_schema::Function;

#[derive(Debug, Default)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    format: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    typecheck: bool,
    /// Only include handlers with these tags (empty means include all).
    include_tags: BTreeSet<String>,
    /// Exclude handlers with these tags (empty means exclude none).
    exclude_tags: BTreeSet<String>,
}

impl Config {
    pub fn format(&mut self, format: bool) -> &mut Self {
        self.format = format;
        self
    }

    pub fn typecheck(&mut self, typecheck: bool) -> &mut Self {
        self.typecheck = typecheck;
        self
    }

    pub fn include_tags(&mut self, include_tags: BTreeSet<String>) -> &mut Self {
        self.include_tags = include_tags;
        self
    }

    pub fn exclude_tags(&mut self, exclude_tags: BTreeSet<String>) -> &mut Self {
        self.exclude_tags = exclude_tags;
        self
    }
}

pub fn generate(mut schema: crate::Schema, config: &Config) -> anyhow::Result<String> {
    let implemented_types = build_implemented_types();

    let mut rendered_types = HashMap::new();
    for original_type_name in schema.consolidate_types() {
        let type_def = schema.get_type(&original_type_name).context(format!(
            "internal error: failed to get consolidated type definition for type: {}",
            original_type_name
        ))?;
        if implemented_types.contains_key(&original_type_name) {
            continue;
        }
        if type_def.as_primitive().map(|i| &i.fallback).is_some() {
            continue;
        }
        rendered_types.insert(
            original_type_name,
            render_type(type_def, &schema, &implemented_types)?,
        );
    }

    let functions_by_name = schema
        .functions()
        .map(|f| (f.name.clone(), f))
        .collect::<IndexMap<_, _>>();
    let function_groups = function_groups_from_function_names(
        schema
            .functions()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>(),
    );

    let mut generated_code = vec![];

    let file_template = templates::FileHeader {
        name: schema.name.clone(),
        description: schema.description.clone(),
    };
    generated_code.push(
        file_template
            .render()
            .context("Failed to render template")?,
    );

    generated_code.push(START_BOILERPLATE.into());
    generated_code.push(include_str!("./lib.ts").to_owned());
    generated_code.push(END_BOILERPLATE.into());

    let mut types = vec![];
    interfaces_from_function_group(
        "",
        &mut types,
        &function_groups,
        &schema,
        &implemented_types,
        &functions_by_name,
    );

    let module = templates::Module {
        name: "__definition".into(),
        types: types.iter().map(|t| t.render().unwrap()).collect(),
        submodules: Default::default(),
    };

    generated_code.push(module.render().context("Failed to render template")?);

    let module = modules_from_rendered_types(schema.consolidate_types(), rendered_types);
    generated_code.push(
        module
            .render()
            .context("Failed to render template")?
            .trim()
            .to_string(),
    );

    let mut rendered_functions = Vec::new();
    for function in schema.functions.iter() {
        rendered_functions.push(render_function(function, &schema, &implemented_types)?);
    }

    let generated_impl_client = client_impl_from_function_group(8, &function_groups).render();
    let file_template = templates::FileFooter {
        start_boilerplate: START_BOILERPLATE,
        end_boilerplate: END_BOILERPLATE,
        client_impl: generated_impl_client,
        implemented_functions: rendered_functions.join("\n"),
    };
    generated_code.push(
        file_template
            .render()
            .context("Failed to render template")?,
    );

    let mut generated_code = generated_code.join("\n");
    if config.format {
        generated_code = format_with(
            // In descending order of speed. The output should be the same.
            [
                Command::new("biome").args(["format", "--stdin-file-path", "dummy.ts"]),
                Command::new("prettier").args(["--parser", "typescript"]),
                Command::new("npx")
                    .arg("prettier")
                    .args(["--parser", "typescript"]),
            ],
            generated_code,
        )?;
    };

    if config.typecheck {
        typecheck(&generated_code)?;
    }

    Ok(generated_code)
}

fn typecheck(src: &str) -> anyhow::Result<()> {
    let path = super::tmp_path(src).with_extension("ts");
    std::fs::write(&path, src)?;

    for cmd in [&mut Command::new("tsc"), Command::new("npx").arg("tsc")] {
        let child = match cmd
            .arg("--noEmit")
            .arg("--skipLibCheck")
            .arg("--strict")
            .args(["--lib", "esnext"])
            .arg(&path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err.into()),
        };

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "tsc failed with exit code {:?}\n{}",
                output.status.code(),
                // tsc outputs to stdout
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        // Remove only after success check to keep file around for debugging
        std::fs::remove_file(&path)?;

        return Ok(());
    }

    Ok(())
}

mod templates {
    use askama::Template;
    use indexmap::IndexMap;

    #[derive(Template)]
    #[template(
        source = "// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: {{ name }}
// {{ description }}

export function client(base: string | Client): __definition.Interface {
    return __implementation.__client(base)
}",
        ext = "txt"
    )]
    pub(super) struct FileHeader {
        pub name: String,
        pub description: String,
    }

    #[derive(Template)]
    #[template(
        source = "
namespace __implementation {

{{ start_boilerplate }}

export function __client(base: string | Client): __definition.Interface {
    const client_instance = typeof base === 'string' ? new ClientInstance(base) : base;
    return { impl: {{ client_impl }} }.impl
}

{{ end_boilerplate }}

{{ implemented_functions }}

}
",
        ext = "txt"
    )]
    pub(super) struct FileFooter {
        pub start_boilerplate: &'static str,
        pub end_boilerplate: &'static str,
        pub client_impl: String,
        pub implemented_functions: String,
    }

    #[derive(Template)]
    #[template(
        source = "
{{ self.render_start() }}
{%- for type in types.iter() %}
{{ type }}
{%- endfor %}
{%- for module in self.submodules_sorted() %}
{{ module }}
{%- endfor %}

{{ self.render_end() }}",
        ext = "txt"
    )]
    pub(super) struct Module {
        pub name: String,
        pub types: Vec<String>,
        pub submodules: IndexMap<String, Module>,
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
                format!("export namespace {} {{", self.name.replace('-', "_"))
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
        source = "
{{ description }}export {{ self.render_keyword() }} {{ name }} {{ self.render_brackets().0 }}
    {%- for field in fields.iter() %}
    {{ field }},
    {%- endfor %}
{{ self.render_brackets().1 }}{{ self.render_flattened_types() }}",
        ext = "txt"
    )]
    pub(super) struct Interface {
        pub name: String,
        pub description: String,
        pub fields: Vec<Field>,
        pub is_tuple: bool,
        pub flattened_types: Vec<String>,
    }

    impl Interface {
        fn render_keyword(&self) -> String {
            if self.is_tuple || !self.flattened_types.is_empty() {
                "type".into()
            } else {
                "interface".into()
            }
        }

        fn render_brackets(&self) -> (&'static str, &'static str) {
            if !self.flattened_types.is_empty() {
                ("= {", "}")
            } else if self.is_tuple {
                ("= [", "]\n")
            } else {
                ("{", "}")
            }
        }

        fn render_flattened_types(&self) -> String {
            if self.flattened_types.is_empty() {
                "".into()
            } else {
                format!(" & {}", self.flattened_types.join(" &\n    "))
            }
        }
    }

    #[derive(Template)]
    #[template(
        source = "
{{ description }}export type {{ name }} =
    {%- for variant in variants.iter() %}
    {{ variant }}
    {%- endfor %};",
        ext = "txt"
    )]
    pub(super) struct Enum {
        pub name: String,
        pub description: String,
        pub variants: Vec<Variant>,
    }

    #[derive(Template)]
    #[template(source = "{{ description }}| {{ self.render_self()? }}", ext = "txt")]
    pub(super) struct Variant {
        pub name: String,
        pub description: String,
        pub representation: crate::Representation,
        pub fields: Fields,
        pub discriminant: Option<isize>,
        pub untagged: bool,
    }

    #[derive(Debug)]
    pub(super) enum Fields {
        Named(Vec<Field>),
        Unnamed(Vec<Field>),
        None,
    }

    impl Fields {
        fn is_empty(&self) -> bool {
            match self {
                Fields::Named(fields) | Fields::Unnamed(fields) => fields.is_empty(),
                Fields::None => true,
            }
        }

        fn len(&self) -> usize {
            match self {
                Fields::Named(fields) | Fields::Unnamed(fields) => fields.len(),
                Fields::None => 0,
            }
        }

        fn is_unnamed(&self) -> bool {
            matches!(self, Fields::Unnamed(_))
        }

        fn iter(&self) -> impl Iterator<Item = &Field> {
            match self {
                Fields::Named(fields) | Fields::Unnamed(fields) => fields.iter(),
                Fields::None => [].iter(),
            }
        }
    }

    impl Variant {
        fn field_brackets(&self) -> (String, String) {
            if self.fields.is_empty() {
                ("".into(), "".into())
            } else if self.fields.is_unnamed() {
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
            if self.untagged {
                return self.render_fields(None);
            }
            let r = match &self.representation {
                crate::Representation::External => {
                    if self.fields.is_empty() {
                        if let Some(discriminant) = self.discriminant {
                            return Ok(format!("{} /* {} */", discriminant, self.name));
                        }

                        return Ok(match self.fields {
                            Fields::Named(_) => format!(r#"{{ {}: {{}} }}"#, self.name),
                            Fields::Unnamed(_) => format!(r#"{{ {}: [] }}"#, self.name),
                            Fields::None => format!(r#""{}""#, self.name),
                        });
                    }

                    format!(
                        "{{\n        {}: {}\n    }}",
                        self.normalized_name(),
                        self.render_fields(None)?
                    )
                }
                crate::Representation::Internal { tag } => {
                    if self.fields.is_empty() {
                        match &self.fields {
                            Fields::Named(_) | Fields::None => {
                                return Ok(format!(r#"{{ {tag}: "{}" }}"#, self.name));
                            }
                            Fields::Unnamed(_) => {
                                unreachable!(
                                    "serde ensures tuple variants can't be internally tagged"
                                )
                            }
                        }
                    }

                    if self.fields.len() == 1 && self.fields.is_unnamed() {
                        format!(
                            r#"{{ {}: "{}" }} & {}"#,
                            tag,
                            self.name,
                            self.render_fields(None)?
                        )
                    } else {
                        self.render_fields(Some(tag))?
                    }
                }
                crate::Representation::Adjacent { tag, content } => {
                    if self.fields.is_empty() {
                        return Ok(match &self.fields {
                            Fields::None | Fields::Named(_) => {
                                format!(r#"{{ {tag}: "{}", {content}: {{}} }}"#, self.name)
                            }
                            Fields::Unnamed(_) => {
                                format!(r#"{{ {tag}: "{}", {content}: [] }}"#, self.name)
                            }
                        });
                    }

                    format!(
                        r#"{{ {}: "{}", {}: {} }}"#,
                        tag,
                        self.normalized_name(),
                        content,
                        self.render_fields(None)?
                    )
                }
                crate::Representation::None => self.render_fields(None)?,
            };
            Ok(r)
        }

        fn normalized_name(&self) -> String {
            if self.name.chars().enumerate().any(|(ind, c)| {
                ind == 0 && !c.is_alphabetic() && c != '_' || !c.is_alphanumeric() && c != '_'
            }) {
                format!("\"{}\"", self.name)
            } else {
                self.name.clone()
            }
        }

        fn render_fields(&self, inner_tag: Option<&str>) -> anyhow::Result<String> {
            let brackets = self.field_brackets();
            if brackets.0.is_empty() && self.fields.is_empty() {
                return Ok(match self.fields {
                    Fields::Named(_) => "{}",
                    Fields::Unnamed(_) => "[]",
                    Fields::None => "null",
                }
                .into());
            }

            let mut rendered_fields = Vec::new();
            if let Some(inner_tag) = inner_tag {
                rendered_fields.push(format!("{}: \"{}\"", inner_tag, self.name));
            }

            for field in self.fields.iter() {
                rendered_fields.push(field.render()?);
            }

            Ok(format!(
                "{}\n            {}\n        {}",
                brackets.0,
                rendered_fields.join(",\n            "),
                brackets.1
            ))
        }
    }

    // TODO The ? is in the wrong place for optional tuple fields.
    // i.e. syntax is { name?: string } for object-like things but [string?] for array-like things
    #[derive(Debug, Template)]
    #[template(
        source = "{{ description }}{% if !self.is_unnamed() %}{{ self.normalized_name() }}{% if optional %}{{ \"?\" }}{% endif %}: {{ type_ }}{% else %}{{ type_ }}{% endif  %}",
        ext = "txt"
    )]
    pub(super) struct Field {
        pub name: String,
        pub description: String,
        pub type_: String,
        pub optional: bool,
    }

    impl Field {
        fn is_unnamed(&self) -> bool {
            self.name.parse::<u64>().is_ok()
        }

        fn normalized_name(&self) -> String {
            if self.name.chars().enumerate().any(|(ind, c)| {
                ind == 0 && !c.is_alphabetic() && c != '_' || !c.is_alphanumeric() && c != '_'
            }) {
                format!("\"{}\"", self.name)
            } else {
                self.name.clone()
            }
        }
    }

    #[derive(Template)]
    #[template(
        source = "
{{ description }}export type {{ name }} = {{ type_ }};",
        ext = "txt"
    )]
    pub(super) struct Alias {
        pub name: String,
        pub description: String,
        pub type_: String,
    }

    #[derive(Template)]
    #[template(
        source = "function {{ name }}(client: Client) {
    return (input: {{ input_type }}, headers: {{ input_headers }}) => __request<
        {{ input_type }}, {{ input_headers }}, {{ output_type }}, {{ error_type }}
    >(client, '{{ path }}', input, headers);
}",
        ext = "txt"
    )]
    pub(super) struct FunctionImplementationTemplate {
        pub name: String,
        pub path: String,
        pub input_type: String,
        pub input_headers: String,
        pub output_type: String,
        pub error_type: String,
    }

    #[derive(Debug)]
    pub(super) struct ClientImplementationGroup {
        pub offset: usize,
        pub functions: IndexMap<String, String>,
        pub subgroups: IndexMap<String, ClientImplementationGroup>,
    }

    impl ClientImplementationGroup {
        fn offset(&self) -> String {
            " ".repeat(self.offset)
        }
        pub fn render(&self) -> String {
            let mut result = vec![];
            result.push("{".to_string());
            for (name, function) in self.functions.iter() {
                result.push(format!(
                    r#"{}"{}": {}(client_instance),"#,
                    self.offset(),
                    name,
                    function
                ));
            }
            for (name, group) in self.subgroups.iter() {
                result.push(format!(
                    r#"{}"{}": {}"#,
                    self.offset(),
                    name,
                    group.render()
                ));
            }
            result.push(format!("{}}},", " ".repeat(self.offset - 4)));
            result.join("\n")
        }
    }
}

struct FunctionGroup {
    functions: Vec<String>,
    subgroups: IndexMap<String, FunctionGroup>,
}

fn function_groups_from_function_names(function_names: Vec<String>) -> FunctionGroup {
    let mut root_group = FunctionGroup {
        functions: vec![],
        subgroups: IndexMap::new(),
    };
    for function_name in function_names {
        let mut group = &mut root_group;
        let mut parts = function_name.split('.').collect::<Vec<_>>();
        parts.pop().unwrap();
        for part in parts {
            group = group.subgroups.entry(part.into()).or_insert(FunctionGroup {
                functions: vec![],
                subgroups: IndexMap::new(),
            });
        }
        group.functions.push(function_name);
    }
    root_group
}

fn client_impl_from_function_group(
    offset: usize,
    group: &FunctionGroup,
) -> templates::ClientImplementationGroup {
    templates::ClientImplementationGroup {
        offset,
        functions: group
            .functions
            .iter()
            .map(|f| {
                (
                    f.split('.').last().unwrap().replace('-', "_"),
                    f.replace('.', "__").replace('-', "_"),
                )
            })
            .collect(),
        subgroups: group
            .subgroups
            .iter()
            .map(|(n, g)| {
                (
                    n.replace('-', "_"),
                    client_impl_from_function_group(offset + 4, g),
                )
            })
            .collect(),
    }
}

fn function_signature(
    function: &Function,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> (String, String, String, String) {
    let input_type = if let Some(input_type) = function.input_type.as_ref() {
        type_ref_to_ts_ref(input_type, schema, implemented_types)
    } else {
        "{}".into()
    };
    let input_headers = if let Some(input_headers) = function.input_headers.as_ref() {
        type_ref_to_ts_ref(input_headers, schema, implemented_types)
    } else {
        "{}".into()
    };
    let output_type = if let Some(output_type) = function.output_type.as_ref() {
        type_ref_to_ts_ref(output_type, schema, implemented_types)
    } else {
        "{}".into()
    };
    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        type_ref_to_ts_ref(error_type, schema, implemented_types)
    } else {
        "{}".into()
    };
    (input_type, input_headers, output_type, error_type)
}

fn interfaces_from_function_group(
    prefix: &str,
    interfaces: &mut Vec<templates::Interface>,
    group: &FunctionGroup,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    functions_by_name: &IndexMap<String, &Function>,
) {
    let mut type_template = templates::Interface {
        name: format!("{prefix}Interface"),
        description: "".into(),
        fields: Default::default(),
        is_tuple: false,
        flattened_types: vec![],
    };

    for function_name in &group.functions {
        let function = functions_by_name.get(function_name).unwrap();
        let (input_type, input_headers, output_type, error_type) =
            function_signature(function, schema, implemented_types);
        type_template.fields.push(templates::Field {
            name: function_name.split('.').last().unwrap().replace('-', "_"),
            description: doc_to_ts_comments(
                &function.description,
                function.deprecation_note.as_deref(),
                4,
            ),
            type_: format!(
                "(input: {}, headers: {})\n        => AsyncResult<{}, {}>",
                input_type, input_headers, output_type, error_type
            ),
            optional: false,
        });
    }

    type_template
        .fields
        .extend(group.subgroups.keys().map(|f| templates::Field {
            name: f.replace('-', "_"),
            description: "".into(),
            type_: format!("{}{}Interface", camelcase(prefix), camelcase(f)),
            optional: false,
        }));

    interfaces.push(type_template);

    for (name, group) in &group.subgroups {
        interfaces_from_function_group(
            &format!("{}{}", camelcase(prefix), camelcase(name)),
            interfaces,
            group,
            schema,
            implemented_types,
            functions_by_name,
        )
    }
}

fn modules_from_rendered_types(
    original_type_names: Vec<String>,
    mut rendered_types: HashMap<String, String>,
) -> templates::Module {
    let mut root_module = templates::Module {
        name: "".into(),
        types: vec![],
        submodules: IndexMap::new(),
    };

    for original_type_name in original_type_names {
        let mut module = &mut root_module;
        let mut parts = original_type_name.split("::").collect::<Vec<_>>();
        parts.pop().unwrap();
        for part in parts {
            module = module
                .submodules
                .entry(part.into())
                .or_insert(templates::Module {
                    name: part.into(),
                    types: vec![],
                    submodules: IndexMap::new(),
                });
        }
        if let Some(rendered_type) = rendered_types.remove(&original_type_name) {
            module.types.push(rendered_type);
        }
    }

    root_module
}

fn render_function(
    function: &Function,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Result<String, anyhow::Error> {
    let (input_type, input_headers, output_type, error_type) =
        function_signature(function, schema, implemented_types);
    let function_template = templates::FunctionImplementationTemplate {
        name: function.name.replace('-', "_").replace('.', "__"),
        path: format!("{}/{}", function.path, function.name),
        input_type,
        input_headers,
        output_type,
        error_type,
    };
    function_template
        .render()
        .context("Failed to render template")
}

fn render_type(
    type_def: &crate::Type,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Result<String, anyhow::Error> {
    let type_name = type_to_ts_name(type_def);

    Ok(match type_def {
        crate::Type::Struct(struct_def) => {
            if struct_def.is_alias() {
                let field_type_ref = struct_def.fields.iter().next().unwrap().type_ref.clone();
                let alias_template = templates::Alias {
                    name: type_name,
                    description: doc_to_ts_comments(&struct_def.description, None, 0),
                    type_: type_ref_to_ts_ref(&field_type_ref, schema, implemented_types),
                };
                alias_template
                    .render()
                    .context("Failed to render template")?
            } else {
                let interface_template = templates::Interface {
                    name: type_name,
                    description: doc_to_ts_comments(&struct_def.description, None, 0),
                    is_tuple: struct_def.is_tuple(),
                    fields: struct_def
                        .fields
                        .iter()
                        .filter(|f| !f.flattened)
                        .map(|f| field_to_ts_field(f, schema, implemented_types))
                        .collect::<Vec<_>>(),
                    flattened_types: struct_def
                        .fields
                        .iter()
                        .filter(|f| f.flattened)
                        .map(|field| {
                            let type_ref =
                                type_ref_to_ts_ref(&field.type_ref, schema, implemented_types);
                            if field.required {
                                // Rust `()` maps to `null` in typescript. This has bad interactions when used with `&`.
                                // Avoid this by turning `null -> {}`.
                                format!("NullToEmptyObject<{type_ref}>")
                            } else {
                                format!("Partial<{}>", type_ref.replace(" | null", ""))
                            }
                        })
                        .collect::<Vec<_>>(),
                };
                interface_template
                    .render()
                    .context("Failed to render template")?
            }
        }
        crate::Type::Enum(enum_def) => {
            let description = doc_to_ts_comments(&enum_def.description, None, 0);

            // An empty enum requires special handling.
            // This is isomorphic to the never type as a value of this type does not exist.
            if enum_def.variants.is_empty() {
                return Ok(format!("{description}export type {type_name} = never"));
            }

            let enum_template = templates::Enum {
                name: type_name,
                description,
                variants: enum_def
                    .variants
                    .iter()
                    .map(|variant| templates::Variant {
                        name: variant.serde_name().into(),
                        description: doc_to_ts_comments(&variant.description, None, 4),
                        representation: enum_def.representation.clone(),
                        fields: fields_to_ts_fields(&variant.fields, schema, implemented_types),
                        discriminant: variant.discriminant,
                        untagged: variant.untagged,
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
            let alias_template = templates::Alias {
                name: type_name,
                description: doc_to_ts_comments(&type_def.description, None, 0),
                type_: format!(
                    "any /* fallback to any for unimplemented type: {} */",
                    type_def.name
                ),
            };
            alias_template
                .render()
                .context("Failed to render template")?
        }
    })
}

fn fields_to_ts_fields(
    fields: &crate::Fields,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> templates::Fields {
    match fields {
        reflectapi_schema::Fields::Named(fields) => templates::Fields::Named(
            fields
                .iter()
                .map(|f| field_to_ts_field(f, schema, implemented_types))
                .collect::<Vec<_>>(),
        ),
        reflectapi_schema::Fields::Unnamed(fields) => templates::Fields::Unnamed(
            fields
                .iter()
                .map(|f| field_to_ts_field(f, schema, implemented_types))
                .collect(),
        ),
        reflectapi_schema::Fields::None => templates::Fields::None,
    }
}

fn field_to_ts_field(
    field: &crate::Field,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> templates::Field {
    templates::Field {
        name: field.serde_name().into(),
        description: doc_to_ts_comments(&field.description, field.deprecation_note.as_deref(), 4),
        type_: type_ref_to_ts_ref(&field.type_ref, schema, implemented_types),
        optional: !field.required,
    }
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
    let n = type_name_parts.join(".");
    let p = type_ref_params_to_ts_ref(&type_ref.arguments, schema, implemented_types);
    format!("{}{}", n, p)
}

fn type_ref_params_to_ts_ref(
    type_params: &[crate::TypeReference],
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

fn resolve_type_ref(
    type_ref: &crate::TypeReference,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Option<String> {
    let Some(mut implementation) = implemented_types.get(type_ref.name.as_str()).cloned() else {
        let Some(fallback_type_ref) = type_ref.fallback_once(schema.input_types()) else {
            let fallback_type_ref = type_ref.fallback_once(schema.output_types())?;
            return Some(type_ref_to_ts_ref(
                &fallback_type_ref,
                schema,
                implemented_types,
            ));
        };

        return Some(type_ref_to_ts_ref(
            &fallback_type_ref,
            schema,
            implemented_types,
        ));
    };

    if type_ref.arguments.is_empty() {
        return Some(implementation);
    }

    let type_def = schema.get_type(type_ref.name())?;

    assert_eq!(type_def.parameters().len(), type_ref.arguments().len());
    for (type_def_param, type_ref_param) in type_def.parameters().zip(type_ref.arguments.iter()) {
        if implementation.contains(type_def_param.name.as_str()) {
            // Ensure only the first occurence of the type parameter is replaced.
            // For example, this can cause trouble for large tuples where T1 erroneously matches T11.
            // I think ideally we shouldn't be doing this by string substitution...
            implementation = implementation.replacen(
                type_def_param.name.as_str(),
                type_ref_to_ts_ref(type_ref_param, schema, implemented_types).as_str(),
                1,
            );
        }
    }

    Some(implementation)
}

fn doc_to_ts_comments(doc: &str, deprecation_note: Option<&str>, offset: u8) -> String {
    let doc = if let Some(note) = deprecation_note {
        if doc.is_empty() {
            Cow::Owned(format!("@deprecated {note}"))
        } else {
            Cow::Owned(format!("@deprecated {note}\n{doc}"))
        }
    } else {
        Cow::Borrowed(doc)
    };

    if doc.is_empty() {
        return "".into();
    }

    let padding = " ".repeat(offset as usize);

    std::iter::once(format!("{padding}/**"))
        .chain(doc.split('\n').map(|s| format!("{padding} * {s}")))
        .chain(std::iter::once(format!("{padding} */\n")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn camelcase(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c == '-' || c == '_' {
            capitalize = true;
        } else if c == '.' {
            result.push('_');
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn build_implemented_types() -> HashMap<String, String> {
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

    // warning: all generic type parameter names should match reflect defnition coming from
    // the implementation of reflect for standard types

    implemented_types.insert("std::option::Option".into(), "(T | null)".into());
    implemented_types.insert("std::array::Array".into(), "FixedSizeArray<T, N>".into());
    implemented_types.insert("reflectapi::Option".into(), "(T | null | undefined)".into());

    implemented_types.insert("std::vec::Vec".into(), "Array<T>".into());
    implemented_types.insert("std::collections::HashMap".into(), "Record<K, V>".into());

    // serde_json serializes `()` as `null` not `[]`
    implemented_types.insert("std::tuple::Tuple0".into(), "null".into());
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

    implemented_types.insert(
        "serde_json::Value".into(),
        "any /* serde_json::Value */".into(),
    );

    // it is only string in json format,
    // message pack delivers it as bytes
    // but we ignore it as this client encodes as only json
    implemented_types.insert("uuid::Uuid".into(), "string /* uuid::Uuid */".into());

    // we preserve it in case the generated code might have references to unused generic parameters
    implemented_types.insert(
        "std::marker::PhantomData".into(),
        "undefined | T /* phantom data */".into(),
    );

    implemented_types
}
