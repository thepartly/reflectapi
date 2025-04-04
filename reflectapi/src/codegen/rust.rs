use std::{
    collections::{BTreeSet, HashMap},
    process::Command,
};

use anyhow::Context;
use askama::Template;
use indexmap::IndexMap;
use reflectapi_schema::Function;

use super::format_with;

#[derive(Debug)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    format: bool,
    /// Include tracing in the generated code.
    instrument: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    typecheck: bool,
    shared_modules: BTreeSet<String>,
    /// Only include handlers with these tags (empty means include all).
    include_tags: BTreeSet<String>,
    /// Exclude handlers with these tags (empty means exclude none).
    exclude_tags: BTreeSet<String>,
    /// Derives to add to all types.
    base_derives: BTreeSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: false,
            instrument: false,
            typecheck: false,
            shared_modules: Default::default(),
            include_tags: Default::default(),
            exclude_tags: Default::default(),
            base_derives: BTreeSet::from_iter(["Debug".into()]),
        }
    }
}

impl Config {
    pub fn format(&mut self, format: bool) -> &mut Self {
        self.format = format;
        self
    }

    pub fn instrument(&mut self, instrument: bool) -> &mut Self {
        self.instrument = instrument;
        self
    }

    pub fn typecheck(&mut self, typecheck: bool) -> &mut Self {
        self.typecheck = typecheck;
        self
    }

    pub fn shared_modules(&mut self, shared_modules: BTreeSet<String>) -> &mut Self {
        self.shared_modules = shared_modules;
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

    pub fn base_derives(&mut self, base_derives: BTreeSet<String>) -> &mut Self {
        self.base_derives = base_derives;
        self
    }
}

pub fn generate(mut schema: crate::Schema, config: &Config) -> anyhow::Result<String> {
    let mut implemented_types = __build_implemented_types();
    for type_def in schema
        .input_types()
        .types()
        .chain(schema.output_types().types())
    {
        if config
            .shared_modules
            .iter()
            .any(|m| type_def.name().starts_with(m))
        {
            implemented_types.insert(
                type_def.name().into(),
                format!(
                    "{}{}",
                    type_def.name(),
                    __type_params_to_ts_name(type_def.parameters())
                ),
            );
        }
    }

    let mut rendered_types = HashMap::new();
    for original_type_name in schema.consolidate_types() {
        if config
            .shared_modules
            .iter()
            .any(|m| original_type_name.starts_with(m))
        {
            continue;
        }
        if original_type_name.starts_with("std::") || original_type_name.starts_with("reflectapi::")
        {
            continue;
        }
        let type_def = schema.get_type(&original_type_name).context(format!(
            "internal error: failed to get consolidated type definition for type: {}",
            original_type_name
        ))?;
        if implemented_types.contains_key(&original_type_name) {
            continue;
        }
        if type_def.as_primitive().is_some() {
            continue;
        }
        rendered_types.insert(
            original_type_name.clone(),
            __render_type(
                config,
                type_def,
                &schema,
                &implemented_types,
                schema.is_input_type(&original_type_name),
                schema.is_output_type(&original_type_name),
            )?,
        );
    }

    let functions_by_name = schema
        .functions()
        .map(|f| (f.name.clone(), f))
        .collect::<IndexMap<_, _>>();
    let function_groups = __function_groups_from_function_names(
        schema
            .functions()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>(),
    );

    let mut generated_code = vec![];

    let file_template = templates::__FileHeader {
        name: schema.name.clone(),
        description: schema.description.clone(),
    };
    generated_code.push(
        file_template
            .render()
            .context("Failed to render template")?,
    );

    let module = __interface_types_from_function_group(
        "".into(),
        &function_groups,
        &schema,
        &implemented_types,
        &functions_by_name,
        config,
    );
    let module = templates::__Module {
        name: "interface".into(),
        types: module,
        submodules: IndexMap::new(),
    };
    generated_code.push(module.render().context("Failed to render template")?);

    let module = __modules_from_rendered_types(schema.consolidate_types(), rendered_types);
    generated_code.push(
        module
            .render()
            .context("Failed to render template")?
            .trim()
            .to_string(),
    );

    let mut generated_code = generated_code.join("\n");

    if config.format {
        generated_code = format_with(
            [Command::new("rustfmt").args(["--edition", "2021"])],
            generated_code,
        )?;
    }

    if config.typecheck {
        typecheck(&generated_code)?;
    }

    Ok(generated_code)
}

fn typecheck(src: &str) -> anyhow::Result<()> {
    // To avoid pulling and compiling expensive dependencies, there are stubs with the relevant type definitions.
    // These are compiled into `rlib` files are used as dependencies for the typechecking process.

    // Make sure the `Makefile` is also kept in sync
    const SOURCES: &[(&str, &str)] = &[
        (
            "serde_derive.rs",
            include_str!("rust-dependency-stubs/serde_derive.rs"),
        ),
        (
            "serde_json.rs",
            include_str!("rust-dependency-stubs/serde_json.rs"),
        ),
        ("serde.rs", include_str!("rust-dependency-stubs/serde.rs")),
        ("bytes.rs", include_str!("rust-dependency-stubs/bytes.rs")),
        ("http.rs", include_str!("rust-dependency-stubs/http.rs")),
        ("chrono.rs", include_str!("rust-dependency-stubs/chrono.rs")),
        (
            "reflectapi.rs",
            include_str!("rust-dependency-stubs/reflectapi.rs"),
        ),
        ("url.rs", include_str!("rust-dependency-stubs/url.rs")),
        (
            "rt.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/rt.rs")),
        ),
        (
            "indexmap.rs",
            include_str!("rust-dependency-stubs/indexmap.rs"),
        ),
        ("Makefile", include_str!("rust-dependency-stubs/Makefile")),
    ];

    let path = super::tmp_path(src);
    std::fs::create_dir_all(&path)?;

    for (name, content) in SOURCES {
        std::fs::write(path.join(name), content)?;
    }

    std::fs::write(path.join("generated.rs"), src)?;

    let output = Command::new("make")
        .current_dir(&path)
        .arg("check")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("rustc typecheck failed (see {}):\n{stderr}", path.display());
    }

    std::fs::remove_dir_all(&path)?;

    Ok(())
}

mod templates {
    use std::collections::BTreeSet;

    use askama::Template;
    use indexmap::IndexMap;

    #[derive(Template)]
    #[template(
        source = "// DO NOT MODIFY THIS FILE MANUALLY
// This file was generated by reflectapi-cli
//
// Schema name: {{ name }}
// {{ description }}

#![allow(non_camel_case_types)]
#![allow(dead_code)]

pub use reflectapi::rt::*;
pub use interface::Interface;",
        ext = "txt"
    )]
    pub(super) struct __FileHeader {
        pub name: String,
        pub description: String,
    }

    #[derive(Template)]
    #[template(
        source = "
{{ self.render_start() }}
{%- for type in types.iter() %}
{{ type }}
{%- endfor %}
{%- for module in self.submodules_sorted() %}
{{- module }}
{%- endfor %}

{{ self.render_end() }}",
        ext = "txt"
    )]
    pub(super) struct __Module {
        pub name: String,
        pub types: Vec<String>,
        pub submodules: IndexMap<String, __Module>,
    }

    impl __Module {
        fn submodules_sorted(&self) -> Vec<&__Module> {
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
                format!("pub mod {} {{", self.name)
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
{{ description }}{{ self.render_attributes_derive() }}
pub struct {{ name }} {{ self.render_brackets().0 }}
    {%- for field in fields.iter() %}
    {{ field }},
    {%- endfor %}
{{ self.render_brackets().1 }}",
        ext = "txt"
    )]
    pub(super) struct __Struct {
        pub name: String,
        pub description: String,
        pub fields: Vec<__Field>,
        pub is_tuple: bool,
        pub is_input_type: bool,
        pub is_output_type: bool,
        pub base_derives: BTreeSet<String>,
        pub codegen_config: reflectapi_schema::RustTypeCodegenConfig,
    }

    impl __Struct {
        fn render_brackets(&self) -> (&'static str, &'static str) {
            if self.is_tuple {
                ("(", ");")
            } else {
                ("{", "}")
            }
        }

        fn render_attributes_derive(&self) -> String {
            let mut attrs = self.codegen_config.additional_derives.clone();
            attrs.extend(self.base_derives.iter().cloned());
            if self.is_input_type {
                // for client it is the inverse, input types are outgoing types
                attrs.insert("serde::Serialize".into());
            }

            if self.is_output_type {
                // for client it is the inverse, output types are incoming types
                attrs.insert("serde::Deserialize".into());
            }

            if attrs.is_empty() {
                "".into()
            } else {
                format!(
                    "#[derive({})]",
                    attrs.into_iter().collect::<Vec<_>>().join(", ")
                )
            }
        }
    }

    #[derive(Template)]
    #[template(
        source = "
{{ description }}{{ self.render_attributes_derive() }}
{{ self.render_attributes() }}pub enum {{ name }} {
    {%- for variant in variants.iter() %}
    {{ variant }}
    {%- endfor %}
}",
        ext = "txt"
    )]
    pub(super) struct __Enum {
        pub name: String,
        pub description: String,
        pub variants: Vec<__Variant>,
        pub representation: crate::Representation,
        pub is_input_type: bool,
        pub is_output_type: bool,
        pub codegen_config: reflectapi_schema::RustTypeCodegenConfig,
        pub base_derives: BTreeSet<String>,
    }

    impl __Enum {
        fn render_attributes_derive(&self) -> String {
            let mut attrs = self.codegen_config.additional_derives.clone();
            attrs.extend(self.base_derives.iter().cloned());
            if self.is_input_type {
                // for client it is the inverse, input types are outgoing types
                attrs.insert("serde::Serialize".into());
            }
            if self.is_output_type {
                // for client it is the inverse, output types are incoming types
                attrs.insert("serde::Deserialize".into());
            }
            if attrs.is_empty() {
                "".into()
            } else {
                format!(
                    "#[derive({})]",
                    attrs.into_iter().collect::<Vec<_>>().join(", ")
                )
            }
        }

        fn render_attributes(&self) -> String {
            let mut attrs = vec![];
            match &self.representation {
                crate::Representation::External => {}
                crate::Representation::Internal { tag } => {
                    attrs.push(format!("tag = \"{}\"", tag));
                }
                crate::Representation::Adjacent { tag, content } => {
                    attrs.push(format!("tag = \"{}\"", tag));
                    attrs.push(format!("content = \"{}\"", content));
                }
                crate::Representation::None => {
                    attrs.push("untagged".into());
                }
            }
            if attrs.is_empty() {
                "".into()
            } else {
                format!("#[serde({})]\n    ", attrs.join(", "))
            }
        }
    }

    #[derive(Template)]
    #[template(
        source = "{{ description }}{{ self.render_attributes() }}{{ self.render_self()? }},",
        ext = "txt"
    )]
    pub(super) struct __Variant {
        pub name: String,
        pub serde_name: String,
        pub description: String,
        pub fields: Vec<__Field>,
        pub discriminant: Option<isize>,
        pub untagged: bool,
    }

    impl __Variant {
        fn render_self(&self) -> anyhow::Result<String> {
            let brakets = self.render_brackets();
            let r = format!(
                "{}{}{}{}{}",
                self.name,
                brakets.0,
                self.render_fields()?,
                brakets.1,
                self.discriminant
                    .map(|d| format!(" = {}", d))
                    .unwrap_or_default()
            );
            Ok(r)
        }

        fn render_attributes(&self) -> String {
            let mut attrs = vec![];
            if self.serde_name != self.name {
                attrs.push(format!("rename = \"{}\"", self.serde_name));
            }
            if self.untagged {
                attrs.push("untagged".into());
            }
            if attrs.is_empty() {
                "".into()
            } else {
                format!("#[serde({})]\n    ", attrs.join(", "))
            }
        }

        fn render_fields(&self) -> anyhow::Result<String> {
            let mut rendered_fields = Vec::new();
            for field in self.fields.iter() {
                rendered_fields.push(field.render()?);
            }
            if rendered_fields.is_empty() {
                Ok("".into())
            } else {
                Ok(format!(
                    "\n        {},\n    ",
                    rendered_fields.join(",\n    ")
                ))
            }
        }

        fn render_brackets(&self) -> (&'static str, &'static str) {
            if self.fields.is_empty() {
                ("", "")
            } else if self.is_tuple() {
                ("(", ")")
            } else {
                (" {", "}")
            }
        }

        fn is_tuple(&self) -> bool {
            self.fields.iter().all(|f| f.is_unnamed())
        }
    }

    #[derive(Template, Clone)]
    #[template(
        source = "{{ description }}{{ self.render_attributes() }}{% if !self.is_unnamed() %}{{ self.render_visibility_modifier() }}{{ name }}: {{ type_ }}{% else %}{{ type_ }}{% endif  %}",
        ext = "txt"
    )]
    pub(super) struct __Field {
        pub name: String,
        pub serde_name: String,
        pub deprecation_note: Option<String>,
        pub description: String,
        pub type_: String,
        pub optional: bool,
        pub flatten: bool,
        pub public: bool,
    }

    impl __Field {
        fn is_unnamed(&self) -> bool {
            self.name.parse::<u64>().is_ok()
        }

        fn render_visibility_modifier(&self) -> String {
            if self.public {
                "pub ".into()
            } else {
                "".into()
            }
        }

        fn render_attributes(&self) -> String {
            let mut serde_attrs = vec![];
            if self.serde_name != self.name {
                serde_attrs.push(format!("rename = \"{}\"", self.serde_name));
            }

            if self.optional {
                serde_attrs.push("default = \"Default::default\"".into());

                // this one is important to not serialize undefined values
                // as this is the special built-in type which allows to differentiate between undefined and null
                if self.type_.starts_with("reflectapi::Option<") {
                    serde_attrs
                        .push("skip_serializing_if = \"reflectapi::Option::is_undefined\"".into());
                }
                // the rest are nice to have, we enumerate only commonly used std types
                if self.type_.starts_with("std::option::Option<") {
                    serde_attrs
                        .push("skip_serializing_if = \"std::option::Option::is_none\"".into());
                }
                if self.type_ == "std::tuple::Tuple0" {
                    serde_attrs.push("skip_serializing".into());
                }
                if self.type_.starts_with("std::string::String") {
                    serde_attrs
                        .push("skip_serializing_if = \"std::string::String::is_empty\"".into());
                }
                if self.type_.starts_with("std::vec::Vec<") {
                    serde_attrs.push("skip_serializing_if = \"std::vec::Vec::is_empty\"".into());
                }
                if self.type_.starts_with("std::collections::") {
                    let type_without_generics = self.type_.split('<').next().unwrap();
                    serde_attrs.push(format!(
                        "skip_serializing_if = \"{}::is_empty\"",
                        type_without_generics
                    ));
                }
            }
            if self.flatten {
                serde_attrs.push("flatten".into());
            }

            let mut out = String::new();
            if !serde_attrs.is_empty() {
                out.push_str("#[serde(");
                out.push_str(&serde_attrs.join(", "));
                out.push_str(")]\n    ");
            }

            if let Some(deprecation_note) = &self.deprecation_note {
                if deprecation_note.is_empty() {
                    out.push_str("#[deprecated]\n    ");
                } else {
                    out.push_str(&format!(
                        "#[deprecated(note = \"{}\")]\n    ",
                        deprecation_note
                    ));
                }
            }

            out
        }
    }

    #[derive(Template)]
    #[template(
        source = "
{{ description }}pub type {{ name }} = {{ type_ }};",
        ext = "txt"
    )]
    pub(super) struct __Alias {
        pub name: String,
        pub description: String,
        pub type_: String,
    }

    #[derive(Template)]
    #[template(
        source = "
{{ description }}{{ self.render_attributes_derive() }}
pub struct {{ name }};",
        ext = "txt"
    )]
    pub(super) struct __Unit {
        pub name: String,
        pub description: String,
        pub is_input_type: bool,
        pub is_output_type: bool,
        pub codegen_config: reflectapi_schema::RustTypeCodegenConfig,
        pub base_derives: BTreeSet<String>,
    }

    impl __Unit {
        fn render_attributes_derive(&self) -> String {
            let mut attrs = self.codegen_config.additional_derives.clone();
            attrs.extend(self.base_derives.iter().cloned());
            if self.is_input_type {
                // for client it is the inverse, input types are outgoing types
                attrs.insert("serde::Serialize".into());
            }
            if self.is_output_type {
                // for client it is the inverse, output types are incoming types
                attrs.insert("serde::Deserialize".into());
            }
            if attrs.is_empty() {
                "".into()
            } else {
                format!(
                    "#[derive({})]",
                    attrs.into_iter().collect::<Vec<_>>().join(", ")
                )
            }
        }
    }

    #[derive(Template)]
    #[template(
        source = r#"
        {%- if let Some(deprecation_note) = deprecation_note -%}
            {%- if deprecation_note.is_empty() -%}
            #[deprecated]
            {%- else -%}
            #[deprecated(note = "{{ deprecation_note }}")]
            {%- endif -%}
        {%- endif -%}
        {{description}}{{attributes}}pub async fn {{ name }}(&self, input: {{ input_type }}, headers: {{ input_headers }})
    -> Result<{{ output_type }}, reflectapi::rt::Error<{{ error_type }}, C::Error>> {
        reflectapi::rt::__request_impl(&self.client, self.base_url.join("{{ path }}").expect("checked base_url already and path is valid"), input, headers).await
    }"#,
        ext = "txt"
    )]
    pub(super) struct __FunctionImplementationTemplate {
        pub name: String,
        pub description: String,
        pub deprecation_note: Option<String>,
        pub attributes: String,
        pub path: String,
        pub input_type: String,
        pub input_headers: String,
        pub output_type: String,
        pub error_type: String,
    }

    #[derive(Template)]
    #[template(
        source = r#"
impl<C: reflectapi::rt::Client + Clone> {{ name }} {
    pub fn try_new(client: C, base_url: reflectapi::rt::Url) -> std::result::Result<Self, reflectapi::rt::UrlParseError> {
        if base_url.cannot_be_a_base() {
            return Err(reflectapi::rt::UrlParseError::RelativeUrlWithCannotBeABaseBase);
        }

        Ok(Self {
            {%- for field in fields.iter() %}
            {{ field.name }}: {{ field.type_ }}::try_new(client.clone(), base_url.clone())?,
            {%- endfor %}
            client,
            base_url,
        })
    }
    {%- for func in functions.iter() %}
    {{ func }}
    {%- endfor %}
}"#,
        ext = "txt"
    )]
    pub(super) struct __InterfaceImplementationTemplate {
        pub name: String,
        pub fields: Vec<__Field>,
        pub functions: Vec<__FunctionImplementationTemplate>,
    }
}

struct __FunctionGroup {
    parent: Vec<String>,
    functions: Vec<String>,
    subgroups: IndexMap<String, __FunctionGroup>,
}

fn __function_groups_from_function_names(function_names: Vec<String>) -> __FunctionGroup {
    let mut root_group = __FunctionGroup {
        parent: vec![],
        functions: vec![],
        subgroups: IndexMap::new(),
    };
    for function_name in function_names {
        let mut group = &mut root_group;
        let mut parts = function_name.split('.').collect::<Vec<_>>();
        parts.pop().unwrap();
        let mut parent = vec![];
        for part in parts {
            group = group
                .subgroups
                .entry(part.into())
                .or_insert(__FunctionGroup {
                    parent: parent.clone(),
                    functions: vec![],
                    subgroups: IndexMap::new(),
                });
            parent.push(part.into());
        }
        group.functions.push(function_name);
    }
    root_group
}

fn __function_signature(
    function: &Function,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> (String, String, String, String) {
    let input_type = if let Some(input_type) = function.input_type.as_ref() {
        __type_ref_to_ts_ref(input_type, schema, implemented_types, 1, None)
    } else {
        "reflectapi::Empty".into()
    };
    let input_headers = if let Some(input_headers) = function.input_headers.as_ref() {
        __type_ref_to_ts_ref(input_headers, schema, implemented_types, 1, None)
    } else {
        "reflectapi::Empty".into()
    };
    let output_type = if let Some(output_type) = function.output_type.as_ref() {
        __type_ref_to_ts_ref(output_type, schema, implemented_types, 1, None)
    } else {
        "reflectapi::Empty".into()
    };
    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        __type_ref_to_ts_ref(error_type, schema, implemented_types, 1, None)
    } else {
        "reflectapi::Empty".into()
    };

    let with_prefix = |name: &str| -> String { name.replace("super::", "super::types::") };

    (
        with_prefix(&input_type),
        with_prefix(&input_headers),
        with_prefix(&output_type),
        with_prefix(&error_type),
    )
}

fn __interface_types_from_function_group(
    name: String,
    group: &__FunctionGroup,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    functions_by_name: &IndexMap<String, &Function>,
    config: &Config,
) -> Vec<String> {
    fn __struct_name_from_parent_name_and_name(parent: &[String], name: &str) -> String {
        if parent.is_empty() {
            return __function_name_for_type_name(name);
        }
        __function_name_for_type_name(&format!("{}_{}", parent.join("_"), name))
    }

    let mut type_template = templates::__Struct {
        name: format!(
            "{}Interface<C: reflectapi::rt::Client + Clone>",
            __struct_name_from_parent_name_and_name(&group.parent, &name)
        ),
        description: "".into(),
        fields: Default::default(),
        is_tuple: false,
        is_input_type: false,
        is_output_type: false,
        codegen_config: Default::default(),
        base_derives: BTreeSet::from_iter(["Debug".into()]),
    };
    let mut interface_implementation = templates::__InterfaceImplementationTemplate {
        name: format!(
            "{}Interface<C>",
            __struct_name_from_parent_name_and_name(&group.parent, &name)
        ),
        fields: vec![],
        functions: vec![],
    };

    for (subgroup_name, subgroup) in group.subgroups.iter() {
        type_template.fields.push(templates::__Field {
            name: __function_name_for_field_name(subgroup_name),
            serde_name: __function_name_for_field_name(subgroup_name),
            deprecation_note: None,
            description: "".into(),
            type_: format!(
                "{}Interface<C>",
                __struct_name_from_parent_name_and_name(&subgroup.parent, subgroup_name)
            ),
            optional: false,
            flatten: false,
            public: true,
        });
        interface_implementation.fields.push(templates::__Field {
            name: __function_name_for_field_name(subgroup_name),
            serde_name: __function_name_for_field_name(subgroup_name),
            deprecation_note: None,
            description: "".into(),
            type_: format!(
                "{}Interface",
                __struct_name_from_parent_name_and_name(&subgroup.parent, subgroup_name)
            ),
            optional: false,
            flatten: false,
            public: true,
        });
    }
    for field in [("client", "C"), ("base_url", "reflectapi::rt::Url")] {
        type_template.fields.push(templates::__Field {
            name: field.0.into(),
            deprecation_note: None,
            serde_name: field.0.into(),
            description: "".into(),
            type_: field.1.into(),
            optional: false,
            flatten: false,
            public: false,
        });
    }

    for function_name in group.functions.iter() {
        let function = functions_by_name.get(function_name).unwrap();
        let (input_type, input_headers, output_type, error_type) =
            __function_signature(function, schema, implemented_types);
        let path = format!("{}/{}", function.path, function.name);
        let func_impl = templates::__FunctionImplementationTemplate {
            name: function
                .name
                .split('.')
                .last()
                .unwrap_or_default()
                .replace('-', "_"),
            deprecation_note: function.deprecation_note.to_owned(),
            // Headers may contain sensitive information, so we skip them
            attributes: if config.instrument {
                format!(r#"#[tracing::instrument(name = "{path}", skip(self, headers))]"#)
            } else {
                String::new()
            },
            description: __doc_to_ts_comments(function.description.as_str(), 4),
            path,
            input_type,
            input_headers,
            output_type,
            error_type,
        };
        interface_implementation.functions.push(func_impl);
    }

    let mut result = vec![
        type_template.render().unwrap(),
        interface_implementation.render().unwrap(),
    ];

    for (subgroup_name, subgroup) in group.subgroups.iter() {
        result.extend(__interface_types_from_function_group(
            subgroup_name.clone(),
            subgroup,
            schema,
            implemented_types,
            functions_by_name,
            config,
        ));
    }
    result
}

fn __modules_from_rendered_types(
    original_type_names: Vec<String>,
    mut rendered_types: HashMap<String, String>,
) -> templates::__Module {
    let mut root_module = templates::__Module {
        name: "types".into(),
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
                .or_insert(templates::__Module {
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

fn __render_type(
    config: &Config,
    type_def: &crate::Type,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    is_input_type: bool,
    is_output_type: bool,
) -> Result<String, anyhow::Error> {
    let type_name = __type_to_ts_name(type_def);
    let type_name_depth = type_def.name().split("::").count() - 1;

    Ok(match type_def {
        crate::Type::Struct(struct_def) => {
            if struct_def.is_unit() {
                let unit_struct_template = templates::__Unit {
                    name: type_name,
                    description: __doc_to_ts_comments(&struct_def.description, 0),
                    is_input_type,
                    is_output_type,
                    codegen_config: struct_def.codegen_config.rust.clone(),
                    base_derives: config.base_derives.clone(),
                };
                unit_struct_template
                    .render()
                    .context("Failed to render template")?
            } else if struct_def.is_alias() {
                let field_type_ref = struct_def.fields.iter().next().unwrap().type_ref.clone();
                let alias_template = templates::__Alias {
                    name: type_name,
                    description: __doc_to_ts_comments(&struct_def.description, 0),
                    type_: __type_ref_to_ts_ref(
                        &field_type_ref,
                        schema,
                        implemented_types,
                        type_name_depth,
                        Some(type_def),
                    ),
                };
                alias_template
                    .render()
                    .context("Failed to render template")?
            } else {
                let interface_template = templates::__Struct {
                    name: type_name,
                    description: __doc_to_ts_comments(&struct_def.description, 0),
                    is_tuple: struct_def.is_tuple(),
                    is_input_type,
                    is_output_type,
                    codegen_config: struct_def.codegen_config.rust.clone(),
                    base_derives: config.base_derives.clone(),
                    fields: struct_def
                        .fields
                        .iter()
                        .map(|field| templates::__Field {
                            name: __field_name_to_snake_case(field.name()),
                            serde_name: field.serde_name().into(),
                            description: __doc_to_ts_comments(&field.description, 4),
                            deprecation_note: field.deprecation_note.clone(),
                            type_: __type_ref_to_ts_ref(
                                &field.type_ref,
                                schema,
                                implemented_types,
                                type_name_depth,
                                Some(type_def),
                            ),
                            optional: !field.required,
                            flatten: field.flattened,
                            public: true,
                        })
                        .collect::<Vec<_>>(),
                };
                interface_template
                    .render()
                    .context("Failed to render template")?
            }
        }
        crate::Type::Enum(enum_def) => {
            let enum_template = templates::__Enum {
                name: type_name,
                description: __doc_to_ts_comments(&enum_def.description, 0),
                representation: enum_def.representation.clone(),
                is_input_type,
                is_output_type,
                codegen_config: enum_def.codegen_config.rust.clone(),
                base_derives: config.base_derives.clone(),
                variants: enum_def
                    .variants
                    .iter()
                    .map(|variant| templates::__Variant {
                        name: __name_to_pascal_case(variant.name()),
                        serde_name: variant.serde_name().into(),
                        description: __doc_to_ts_comments(&variant.description, 4),
                        fields: variant
                            .fields
                            .iter()
                            .map(|field| templates::__Field {
                                name: __field_name_to_snake_case(field.name()),
                                serde_name: field.serde_name().into(),
                                description: __doc_to_ts_comments(&field.description, 12),
                                deprecation_note: field.deprecation_note.clone(),
                                type_: __type_ref_to_ts_ref(
                                    &field.type_ref,
                                    schema,
                                    implemented_types,
                                    type_name_depth,
                                    Some(type_def),
                                ),
                                optional: !field.required,
                                flatten: field.flattened,
                                public: false,
                            })
                            .collect::<Vec<_>>(),
                        discriminant: variant.discriminant,
                        untagged: variant.untagged,
                    })
                    .collect::<Vec<_>>(),
            };
            enum_template
                .render()
                .context("Failed to render template")?
        }
        crate::Type::Primitive(_) => {
            // do nothing, we will use the primitive types as is
            "".into()
        }
    })
}

fn __type_ref_to_ts_ref(
    type_ref: &crate::TypeReference,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    type_name_depth: usize,
    parent: Option<&crate::Type>,
) -> String {
    let with_super_prefix =
        |name: &str| -> String { format!("{}{}", "super::".repeat(type_name_depth), name) };

    if let Some(resolved_type) =
        __resolve_type_ref(type_ref, schema, implemented_types, type_name_depth, parent)
    {
        return resolved_type;
    }

    let mut prefix = schema
        .get_type(type_ref.name())
        .filter(|i| !i.is_primitive())
        .map(|_| with_super_prefix(""))
        .unwrap_or_default();

    if type_ref.name().starts_with("reflectapi::") {
        prefix = "".into();
    }

    format!(
        "{}{}{}",
        prefix,
        type_ref.name,
        __type_args_to_ts_ref(
            &type_ref.arguments,
            schema,
            implemented_types,
            type_name_depth,
            parent
        )
    )
}

fn __type_args_to_ts_ref(
    type_params: &[crate::TypeReference],
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    type_name_depth: usize,
    parent: Option<&crate::Type>,
) -> String {
    let p = type_params
        .iter()
        .map(|type_ref| {
            __type_ref_to_ts_ref(type_ref, schema, implemented_types, type_name_depth, parent)
        })
        .collect::<Vec<_>>()
        .join(", ");
    if p.is_empty() {
        p
    } else {
        format!("<{}>", p)
    }
}

fn __type_to_ts_name(type_def: &crate::Type) -> String {
    let n = type_def
        .name()
        .split("::")
        .last()
        .unwrap_or_default()
        .to_string();
    let p = __type_params_to_ts_name(type_def.parameters());
    format!("{}{}", n, p)
}

fn __type_params_to_ts_name(type_params: std::slice::Iter<'_, crate::TypeParameter>) -> String {
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

fn __resolve_type_ref(
    type_ref: &crate::TypeReference,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
    type_name_depth: usize,
    parent: Option<&crate::Type>,
) -> Option<String> {
    let type_def = schema.get_type(type_ref.name())?;

    if let Some(parent) = parent {
        if type_ref.arguments.is_empty() && parent.parameters().any(|p| p.name() == type_ref.name) {
            // This is a reference to a type parameter of the containing type
            return Some(type_ref.name.clone());
        }
    }

    let mut implementation = implemented_types.get(type_ref.name.as_str()).cloned()?;
    for (type_def_param, type_ref_param) in type_def.parameters().zip(type_ref.arguments.iter()) {
        if implementation.contains(type_def_param.name.as_str()) {
            implementation = implementation.replacen(
                type_def_param.name.as_str(),
                __type_ref_to_ts_ref(
                    type_ref_param,
                    schema,
                    implemented_types,
                    type_name_depth,
                    parent,
                )
                .as_str(),
                1,
            );
        }
    }

    Some(implementation)
}

fn __doc_to_ts_comments(doc: &str, offset: u8) -> String {
    if doc.is_empty() {
        return "".into();
    }

    let offset = " ".repeat(offset as usize);
    let doc = doc.split('\n').collect::<Vec<_>>();
    let sp = if doc.iter().all(|i| i.starts_with(' ')) {
        ""
    } else {
        " "
    };
    let doc = doc
        .iter()
        .map(|line| format!("///{}{}", sp, line.trim_end()))
        .collect::<Vec<_>>()
        .join(format!("\n{}", offset).as_str());
    format!("{}\n{}", doc, offset)
}

fn __build_implemented_types() -> HashMap<String, String> {
    let mut implemented_types = HashMap::new();

    // TODO once the todos below are addressed it would be possible to drop this function completely

    // warning: all generic type parameter names should match reflect defnition coming from
    // the implementation of reflect for standard types

    // TODO this one should probably be defined as primitive type
    implemented_types.insert(
        "std::option::Option".into(),
        "std::option::Option<T>".into(),
    );
    // TODO this one should probably be defined as primitive type
    implemented_types.insert("reflectapi::Option".into(), "reflectapi::Option<T>".into());

    implemented_types.insert("std::array::Array".into(), "[T; N]".into());

    // TODO the following could be declared via type aliases in the generated code or in the reflect api
    implemented_types.insert("std::tuple::Tuple0".into(), "()".into());
    implemented_types.insert("std::tuple::Tuple1".into(), "(T1)".into());
    implemented_types.insert("std::tuple::Tuple2".into(), "(T1, T2)".into());
    implemented_types.insert("std::tuple::Tuple3".into(), "(T1, T2, T3)".into());
    implemented_types.insert("std::tuple::Tuple4".into(), "(T1, T2, T3, T4)".into());
    implemented_types.insert("std::tuple::Tuple5".into(), "(T1, T2, T3, T4, T5)".into());
    implemented_types.insert(
        "std::tuple::Tuple6".into(),
        "(T1, T2, T3, T4, T5, T6)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple7".into(),
        "(T1, T2, T3, T4, T5, T6, T7)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple8".into(),
        "(T1, T2, T3, T4, T5, T6, T7, T8)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple9".into(),
        "(T1, T2, T3, T4, T5, T6, T7, T8, T9)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple10".into(),
        "(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple11".into(),
        "(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)".into(),
    );
    implemented_types.insert(
        "std::tuple::Tuple12".into(),
        "(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)".into(),
    );

    implemented_types.insert("std::time::Duration".into(), "std::time::Duration".into());

    implemented_types
}

fn __function_name_for_type_name(name: &str) -> String {
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

fn __function_name_for_field_name(name: &str) -> String {
    name.replace('-', "_")
}

fn __name_to_pascal_case(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' {
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

fn __field_name_to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    // if rust keyword, add underscore
    if check_keyword::CheckKeyword::is_keyword(&result) {
        format!("{}_", result)
    } else {
        result
    }
}
