use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    process::{Command, Stdio},
};

use super::{format_with, END_BOILERPLATE, START_BOILERPLATE};
use anyhow::Context;
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

pub fn generate(
    mut schema: crate::Schema,
    config: &Config,
) -> anyhow::Result<BTreeMap<String, String>> {
    schema.strip_hidden_fields();
    super::validate_response_enums(&schema)?;
    let implemented_types = build_implemented_types();

    let mut rendered_types = HashMap::new();
    for original_type_name in schema.consolidate_types() {
        let type_def = schema.get_type(&original_type_name).context(format!(
            "internal error: failed to get consolidated type definition for type: {original_type_name}"
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

    for function in schema.functions() {
        if let Some(type_ref) = function.response_headers() {
            if !matches!(
                schema.get_type(&type_ref.name),
                Some(crate::Type::Struct(_))
            ) {
                anyhow::bail!(
                    "response headers type `{}` of function `{}` must be a struct",
                    type_ref.name,
                    function.name
                );
            }
        }
    }

    let functions_by_name = schema
        .functions()
        .map(|f| (f.name.clone(), f))
        .collect::<IndexMap<_, _>>();
    let needs_decoders = schema
        .functions()
        .any(|function| function_has_decoder(function, &schema));
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
    generated_code.push(file_template.render());

    generated_code.push(START_BOILERPLATE.into());
    generated_code.push(include_str!("./lib.ts").to_owned());
    if needs_decoders {
        generated_code.push(include_str!("./lib_decode.ts").to_owned());
    }
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
        types: types.iter().map(|t| t.render()).collect(),
        submodules: Default::default(),
    };

    generated_code.push(module.render());

    let module = modules_from_rendered_types(schema.consolidate_types(), rendered_types);
    generated_code.push(module.render().trim().to_string());

    if needs_decoders {
        generated_code.push(render_decoder_specs(&schema));
    }

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
    generated_code.push(file_template.render());

    let mut main = generated_code.join("\n");
    let mut transport = format!(
        "{}\n{}",
        templates::header_comment(&schema.name, &schema.description),
        include_str!("./lib_transport.ts"),
    );
    if config.format {
        main = format_ts(main)?;
        transport = format_ts(transport)?;
    };

    if config.typecheck {
        typecheck(&main, &transport)?;
    }

    let mut files = BTreeMap::new();
    files.insert("generated.ts".to_string(), main);
    files.insert("generated.transport.ts".to_string(), transport);
    Ok(files)
}

fn format_ts(src: String) -> anyhow::Result<String> {
    // NOTE: When updating the biome version, also update .github/workflows/ci.yaml
    let biome_package = "@biomejs/biome@1.8.3";
    Ok(format_with(
        // In descending order of speed. The output should be the same.
        [
            Command::new("biome").args(["format", "--stdin-file-path", "dummy.ts"]),
            Command::new("npx").arg("-y").arg(biome_package).args([
                "format",
                "--stdin-file-path",
                "dummy.ts",
            ]),
            Command::new("prettier").args(["--parser", "typescript"]),
            Command::new("npx")
                .arg("-y")
                .arg("prettier")
                .args(["--parser", "typescript"]),
        ],
        src,
    )?)
}

fn typecheck(main: &str, transport: &str) -> anyhow::Result<()> {
    // tsc resolves `import ... from './generated.transport'` against
    // the filesystem, so both files must live in the same directory.
    let dir = super::tmp_path(main).with_extension("dir");
    std::fs::create_dir_all(&dir)?;
    let main_path = dir.join("generated.ts");
    let transport_path = dir.join("generated.transport.ts");
    std::fs::write(&main_path, main)?;
    std::fs::write(&transport_path, transport)?;

    for cmd in [
        &mut Command::new("tsc"),
        Command::new("npx")
            .arg("-y")
            .arg("-p")
            .arg("typescript")
            .arg("tsc"),
    ] {
        let child = match cmd
            .arg("--noEmit")
            .arg("--skipLibCheck")
            .arg("--strict")
            .args(["--lib", "esnext,DOM"])
            .arg(&main_path)
            .arg(&transport_path)
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
                "tsc failed with exit code {:?}\n{}\n{}",
                output.status.code(),
                // tsc outputs to stdout
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }

        // Remove only after success check to keep files around for debugging
        let _ = std::fs::remove_dir_all(&dir);

        return Ok(());
    }

    Ok(())
}

mod templates {
    use indexmap::IndexMap;
    use std::fmt::Write;

    pub(super) struct FileHeader {
        pub name: String,
        pub description: String,
    }

    /// Render just the `// ...` header comment block that marks a file as
    /// generated. Shared between `generated.ts` and `generated.transport.ts`
    /// so every file the CLI writes carries the same provenance marker.
    pub(in crate::codegen) fn header_comment(name: &str, description: &str) -> String {
        let description_line = if description.is_empty() {
            String::new()
        } else {
            format!("// {description}\n")
        };
        format!(
            "// DO NOT MODIFY THIS FILE MANUALLY\n\
             // This file was generated by reflectapi-cli\n\
             //\n\
             // Schema name: {name}\n\
             {description_line}"
        )
    }

    impl FileHeader {
        pub fn render(&self) -> String {
            format!(
                "{}\n\
                 export function client(base: string | Client): __definition.Interface {{\n\
                     return __implementation.__client(base)\n\
                 }}",
                header_comment(&self.name, &self.description)
            )
        }
    }

    pub(super) struct FileFooter {
        pub start_boilerplate: &'static str,
        pub end_boilerplate: &'static str,
        pub client_impl: String,
        pub implemented_functions: String,
    }

    impl FileFooter {
        pub fn render(&self) -> String {
            format!(
                "\nnamespace __implementation {{\n\
                 \n\
                 {}\n\
                 \n\
                 export function __client(base: string | Client): __definition.Interface {{\n\
                     const client_instance = typeof base === 'string' ? new ClientInstance(base) : base;\n\
                     return {{ impl: {} }}.impl\n\
                 }}\n\
                 \n\
                 {}\n\
                 \n\
                 {}\n\
                 \n\
                 }}\n",
                self.start_boilerplate,
                self.client_impl,
                self.end_boilerplate,
                self.implemented_functions,
            )
        }
    }

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

        pub fn render(&self) -> String {
            let mut out = String::new();
            if !self.name.is_empty() && !self.is_empty() {
                write!(out, "\nexport namespace {} {{", self.name.replace('-', "_")).unwrap();
            }
            for t in &self.types {
                write!(out, "\n{t}").unwrap();
            }
            for module in self.submodules_sorted() {
                out.push('\n');
                out.push_str(&module.render());
            }
            if !self.name.is_empty() && !self.is_empty() {
                write!(out, "\n\n}}").unwrap();
            }
            out
        }
    }

    pub(super) struct Interface {
        pub name: String,
        pub description: String,
        pub fields: Vec<Field>,
        pub is_tuple: bool,
        pub flattened_types: Vec<String>,
    }

    impl Interface {
        fn render_keyword(&self) -> &'static str {
            if self.is_tuple || !self.flattened_types.is_empty() {
                "type"
            } else {
                "interface"
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

        pub fn render(&self) -> String {
            let brackets = self.render_brackets();
            let mut out = format!(
                "\n{}export {} {} {}",
                self.description,
                self.render_keyword(),
                self.name,
                brackets.0,
            );
            for field in &self.fields {
                write!(out, "\n    {},", field.render()).unwrap();
            }
            write!(out, "\n{}{}", brackets.1, self.render_flattened_types()).unwrap();
            out
        }
    }

    pub(super) struct Enum {
        pub name: String,
        pub description: String,
        pub variants: Vec<Variant>,
        pub open: bool,
    }

    impl Enum {
        pub fn render(&self) -> anyhow::Result<String> {
            let mut out = format!("\n{}export type {} =", self.description, self.name,);
            for variant in &self.variants {
                write!(out, "\n    {}", variant.render()?).unwrap();
            }
            if self.open {
                out.push_str("\n    | UnknownEnum");
            }
            out.push(';');
            Ok(out)
        }
    }

    pub(super) struct Variant {
        pub name: String,
        pub description: String,
        pub deprecation_message: Option<String>,
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
        pub fn render(&self) -> anyhow::Result<String> {
            Ok(format!("{}| {}", self.description, self.render_self()?))
        }

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

                    // Also on the tag key, so accessing `value.Variant` is
                    // flagged, not just hovering the union member.
                    let key_doc = self
                        .deprecation_message
                        .as_deref()
                        .map(|message| super::doc_to_ts_comments("", Some(message), 8))
                        .unwrap_or_default();
                    format!(
                        "{{\n{key_doc}        {}: {}\n    }}",
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

                    match &self.fields {
                        Fields::Unnamed(fields) if fields.len() == 1 => {
                            if fields[0].type_ == "null" {
                                format!(r#"{{ {tag}: "{}" }}"#, self.name)
                            } else {
                                format!(
                                    r#"{{ {tag}: "{}" }} & {}"#,
                                    self.name,
                                    self.render_fields(None)?
                                )
                            }
                        }
                        _ => self.render_fields(Some(tag))?,
                    }
                }
                crate::Representation::Adjacent { tag, content } => {
                    if self.fields.is_empty() {
                        return Ok(match &self.fields {
                            Fields::None => format!(r#"{{ {tag}: "{}" }}"#, self.name),
                            Fields::Named(_) => {
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
                rendered_fields.push(field.render());
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
    #[derive(Debug)]
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

        pub fn render(&self) -> String {
            if self.is_unnamed() {
                format!("{}{}", self.description, self.type_)
            } else if self.optional {
                format!(
                    "{}{}?: {}",
                    self.description,
                    self.normalized_name(),
                    self.type_
                )
            } else {
                format!(
                    "{}{}: {}",
                    self.description,
                    self.normalized_name(),
                    self.type_
                )
            }
        }
    }

    pub(super) struct Alias {
        pub name: String,
        pub description: String,
        pub type_: String,
    }

    impl Alias {
        pub fn render(&self) -> String {
            format!(
                "\n{}export type {} = {};",
                self.description, self.name, self.type_
            )
        }
    }

    pub(super) struct FunctionImplementationTemplate {
        pub name: String,
        pub path: String,
        pub input_type: String,
        pub input_headers: String,
        pub output_type: String,
        pub error_type: String,
        pub response_headers: Option<super::ResponseHeaders>,
        pub decoder_args: String,
    }

    impl FunctionImplementationTemplate {
        pub fn render(&self) -> String {
            let response_headers_type = self
                .response_headers
                .as_ref()
                .map(super::ResponseHeaders::type_arg)
                .unwrap_or_default();
            let response_headers_arg = self
                .response_headers
                .as_ref()
                .map(super::ResponseHeaders::call_arg)
                .unwrap_or_default();
            format!(
                "function {name}(client: Client) {{\n\
                     return (input: {input_type}, headers: {input_headers}, options?: RequestOptions) => __request<\n\
                         {input_type}, {input_headers}, {output_type}, {error_type}{response_headers_type}\n\
                     >(client, '{path}', input, headers, options{response_headers_arg}{decoder_args});\n\
                 }}",
                name = self.name,
                input_type = self.input_type,
                input_headers = self.input_headers,
                output_type = self.output_type,
                error_type = self.error_type,
                path = self.path,
                decoder_args = self.decoder_args,
            )
        }
    }

    pub(super) struct StreamFunctionImplementationTemplate {
        pub name: String,
        pub path: String,
        pub input_type: String,
        pub input_headers: String,
        pub item_type: String,
        pub error_type: String,
        pub response_headers: Option<super::ResponseHeaders>,
        pub decoder_args: String,
    }

    impl StreamFunctionImplementationTemplate {
        pub fn render(&self) -> String {
            let response_headers_type = self
                .response_headers
                .as_ref()
                .map(super::ResponseHeaders::type_arg)
                .unwrap_or_default();
            let response_headers_arg = self
                .response_headers
                .as_ref()
                .map(super::ResponseHeaders::call_arg)
                .unwrap_or_default();
            format!(
                "function {name}(client: Client) {{\n\
                     return (input: {input_type}, headers: {input_headers}, options?: RequestOptions) => __stream_request<\n\
                         {input_type}, {input_headers}, {item_type}, {error_type}{response_headers_type}\n\
                     >(client, '{path}', input, headers, options{response_headers_arg}{decoder_args});\n\
                 }}",
                name = self.name,
                input_type = self.input_type,
                input_headers = self.input_headers,
                item_type = self.item_type,
                error_type = self.error_type,
                path = self.path,
                decoder_args = self.decoder_args,
            )
        }
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
                    f.split('.').next_back().unwrap().replace('-', "_"),
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

enum FunctionOutput {
    Single { output_type: String },
    Stream { item_type: String },
}

struct FunctionSignature {
    input_type: String,
    input_headers: String,
    output: FunctionOutput,
    error_type: String,
    response_headers: Option<ResponseHeaders>,
}

/// A function's declared response headers: the TypeScript type of the
/// headers struct, and the header names the runtime reads.
struct ResponseHeaders {
    type_: String,
    names: Vec<String>,
}

impl ResponseHeaders {
    /// Trailing type argument for `AsyncResult` / `__request`.
    fn type_arg(&self) -> String {
        format!(", {}", self.type_)
    }

    /// Trailing call argument for `__request` / `__stream_request`.
    fn call_arg(&self) -> String {
        let names = serde_json::to_string(&self.names).expect("header names serialize");
        format!(", {names}")
    }
}

impl FunctionSignature {
    fn interface_return_type(&self) -> String {
        let output_type = match &self.output {
            FunctionOutput::Single { output_type } => output_type.clone(),
            FunctionOutput::Stream { item_type } => format!("AsyncIterable<{item_type}>"),
        };
        let response_headers_type = self
            .response_headers
            .as_ref()
            .map(ResponseHeaders::type_arg)
            .unwrap_or_default();
        format!(
            "AsyncResult<{output_type}, {error_type}{response_headers_type}>",
            error_type = self.error_type
        )
    }
}

fn function_signature(
    function: &Function,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> FunctionSignature {
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
    let output = match &function.output_type {
        reflectapi_schema::OutputType::Complete { output_type } => FunctionOutput::Single {
            output_type: output_type
                .as_ref()
                .map(|t| type_ref_to_ts_ref(t, schema, implemented_types))
                .unwrap_or_else(|| "{}".into()),
        },
        reflectapi_schema::OutputType::Stream { item_type } => FunctionOutput::Stream {
            item_type: type_ref_to_ts_ref(item_type, schema, implemented_types),
        },
    };
    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        type_ref_to_ts_ref(error_type, schema, implemented_types)
    } else {
        "{}".into()
    };
    let response_headers = function.response_headers.as_ref().map(|response_headers| {
        let Some(crate::Type::Struct(s)) = schema.get_type(&response_headers.name) else {
            unreachable!("response headers types are checked in `generate`")
        };
        let names = s
            .fields()
            .map(|field| field.serde_name().to_owned())
            .collect();
        ResponseHeaders {
            type_: type_ref_to_ts_ref(response_headers, schema, implemented_types),
            names,
        }
    });
    FunctionSignature {
        input_type,
        input_headers,
        output,
        error_type,
        response_headers,
    }
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
        let sig = function_signature(function, schema, implemented_types);
        let return_type = sig.interface_return_type();
        type_template.fields.push(templates::Field {
            name: function_name.split('.').next_back().unwrap().replace('-', "_"),
            description: doc_to_ts_comments(
                &function.description,
                function.deprecation().map(|d| d.message()).as_deref(),
                4,
            ),
            type_: format!(
                "(input: {input_type}, headers: {input_headers}, options?: RequestOptions)\n        => {return_type}",
                input_type = sig.input_type,
                input_headers = sig.input_headers,
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

fn output_type<'a>(schema: &'a crate::Schema, name: &str) -> Option<&'a crate::Type> {
    schema
        .output_types()
        .get_type(name)
        .or_else(|| schema.input_types().get_type(name))
}

fn function_has_decoder(function: &Function, schema: &crate::Schema) -> bool {
    let output = match &function.output_type {
        reflectapi_schema::OutputType::Complete { output_type } => output_type.as_ref(),
        reflectapi_schema::OutputType::Stream { item_type } => Some(item_type),
    };
    output.is_some_and(|r| reaches_decoded_enum(r, schema, &mut BTreeSet::new()))
        || function
            .error_type
            .as_ref()
            .is_some_and(|r| reaches_decoded_enum(r, schema, &mut BTreeSet::new()))
        || function
            .response_headers
            .as_ref()
            .is_some_and(|r| reaches_decoded_enum(r, schema, &mut BTreeSet::new()))
}

fn reaches_decoded_enum(
    reference: &crate::TypeReference,
    schema: &crate::Schema,
    visiting: &mut BTreeSet<String>,
) -> bool {
    if reference
        .arguments
        .iter()
        .any(|argument| reaches_decoded_enum(argument, schema, visiting))
    {
        return true;
    }
    if !visiting.insert(reference.name.clone()) {
        return false;
    }
    let result = match output_type(schema, &reference.name) {
        Some(crate::Type::Enum(e)) => {
            e.open
                || e.variants.iter().any(|v| v.other)
                || e.variants.iter().any(|v| {
                    v.fields
                        .iter()
                        .any(|f| reaches_decoded_enum(&f.type_ref, schema, visiting))
                })
        }
        Some(crate::Type::Struct(s)) => s
            .fields
            .iter()
            .any(|f| reaches_decoded_enum(&f.type_ref, schema, visiting)),
        Some(crate::Type::Primitive(_)) => reference
            .fallback_once(schema.output_types())
            .is_some_and(|fallback| reaches_decoded_enum(&fallback, schema, visiting)),
        None => false,
    };
    visiting.remove(&reference.name);
    result
}

fn decoder_for_ref(reference: &crate::TypeReference, schema: &crate::Schema) -> Option<String> {
    reaches_decoded_enum(reference, schema, &mut BTreeSet::new()).then(|| {
        format!(
            "(__value: unknown) => __decode(__value, {}, __decoder_specs)",
            decoder_ref_spec(reference, schema, &HashMap::new())
        )
    })
}

fn ts_quote(value: &str) -> String {
    serde_json::to_string(value).expect("string serializes")
}

fn decoder_ref_spec(
    reference: &crate::TypeReference,
    schema: &crate::Schema,
    parameters: &HashMap<String, usize>,
) -> String {
    if let Some(index) = parameters.get(&reference.name) {
        return format!("args[{index}]");
    }
    let args = &reference.arguments;
    let argument = |index: usize| {
        args.get(index)
            .map(|r| decoder_ref_spec(r, schema, parameters))
            .unwrap_or_else(|| "\"pass\"".into())
    };
    match reference.name.as_str() {
        "bool" => return "\"boolean\"".into(),
        "char" | "std::string::String" | "uuid::Uuid" => return "\"string\"".into(),
        "std::option::Option" | "reflectapi::Option" => {
            return format!("{{k:\"nullable\",v:{}}}", argument(0));
        }
        "std::vec::Vec" | "std::array::Array" => {
            return format!("{{k:\"array\",v:{}}}", argument(0));
        }
        "std::collections::HashMap" | "std::collections::BTreeMap" => {
            return format!("{{k:\"map\",v:{}}}", argument(1));
        }
        "std::tuple::Tuple0" => return "\"null\"".into(),
        "serde_json::Value" | "std::marker::PhantomData" => return "\"pass\"".into(),
        "u8" => return "{k:\"integer\",min:0,max:255}".into(),
        "u16" => return "{k:\"integer\",min:0,max:65535}".into(),
        "u32" => return "{k:\"integer\",min:0,max:4294967295}".into(),
        "u64" | "u128" => return "{k:\"integer\",min:0}".into(),
        "i8" => return "{k:\"integer\",min:-128,max:127}".into(),
        "i16" => return "{k:\"integer\",min:-32768,max:32767}".into(),
        "i32" => return "{k:\"integer\",min:-2147483648,max:2147483647}".into(),
        "i64" | "i128" => return "{k:\"integer\"}".into(),
        "f32" | "f64" => return "\"number\"".into(),
        name if name.starts_with("std::tuple::Tuple") => {
            return format!(
                "{{k:\"tuple\",v:[{}]}}",
                args.iter()
                    .map(|r| decoder_ref_spec(r, schema, parameters))
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        _ => {}
    }
    if let Some(crate::Type::Primitive(_)) = output_type(schema, &reference.name) {
        if let Some(fallback) = reference.fallback_once(schema.output_types()) {
            return decoder_ref_spec(&fallback, schema, parameters);
        }
        return "\"pass\"".into();
    }
    if output_type(schema, &reference.name).is_none() {
        return "\"pass\"".into();
    }
    let args = args
        .iter()
        .map(|r| decoder_ref_spec(r, schema, parameters))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{k:\"ref\",n:{},a:[{args}]}}", ts_quote(&reference.name))
}

fn decoder_fields_spec(
    fields: &crate::Fields,
    schema: &crate::Schema,
    parameters: &HashMap<String, usize>,
) -> String {
    match fields {
        crate::Fields::None => "\"pass\"".into(),
        crate::Fields::Named(fields) => decoder_object_spec(fields, schema, parameters),
        crate::Fields::Unnamed(fields) => {
            if fields.len() == 1 {
                decoder_ref_spec(&fields[0].type_ref, schema, parameters)
            } else {
                format!(
                    "{{k:\"tuple\",v:[{}]}}",
                    fields
                        .iter()
                        .map(|f| decoder_ref_spec(&f.type_ref, schema, parameters))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }
}

fn decoder_object_spec(
    fields: &[crate::Field],
    schema: &crate::Schema,
    parameters: &HashMap<String, usize>,
) -> String {
    let ordinary = fields
        .iter()
        .filter(|f| !f.flattened)
        .map(|field| {
            format!(
                "[{},{},{}]",
                ts_quote(field.serde_name()),
                decoder_ref_spec(&field.type_ref, schema, parameters),
                field.required
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let flattened = fields
        .iter()
        .filter(|f| f.flattened)
        .map(|field| decoder_ref_spec(&field.type_ref, schema, parameters))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{k:\"object\",f:[{ordinary}],x:[{flattened}]}}")
}

fn decoder_type_spec(
    type_def: &crate::Type,
    schema: &crate::Schema,
    parameters: &HashMap<String, usize>,
) -> String {
    match type_def {
        crate::Type::Struct(s) if s.is_alias() => {
            decoder_ref_spec(&s.fields[0].type_ref, schema, parameters)
        }
        crate::Type::Struct(s) if s.is_tuple() => {
            format!(
                "{{k:\"tuple\",v:[{}]}}",
                s.fields
                    .iter()
                    .map(|f| decoder_ref_spec(&f.type_ref, schema, parameters))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        crate::Type::Struct(s) => match &s.fields {
            crate::Fields::Named(fields) => decoder_object_spec(fields, schema, parameters),
            crate::Fields::Unnamed(fields) => format!(
                "{{k:\"tuple\",v:[{}]}}",
                fields
                    .iter()
                    .map(|f| decoder_ref_spec(&f.type_ref, schema, parameters))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            crate::Fields::None => decoder_object_spec(&[], schema, parameters),
        },
        crate::Type::Enum(e) => {
            let (repr, tag, content) = match &e.representation {
                crate::Representation::External => ("external", "", ""),
                crate::Representation::Internal { tag } => ("internal", tag.as_str(), ""),
                crate::Representation::Adjacent { tag, content } => {
                    ("adjacent", tag.as_str(), content.as_str())
                }
                crate::Representation::None => {
                    let variants = e
                        .variants
                        .iter()
                        .map(|variant| match &variant.fields {
                            crate::Fields::None => "\"null\"".into(),
                            fields => decoder_fields_spec(fields, schema, parameters),
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    return format!("{{k:\"untagged\",v:[{variants}]}}");
                }
            };
            let fallback = e
                .variants
                .iter()
                .find(|v| v.other)
                .map(|v| ts_quote(v.serde_name()))
                .unwrap_or_else(|| "null".into());
            let variants = e
                .variants
                .iter()
                .map(|variant| {
                    let style = match &variant.fields {
                        crate::Fields::None => "none",
                        crate::Fields::Named(_) => "named",
                        crate::Fields::Unnamed(_) => "unnamed",
                    };
                    format!(
                        "{{t:{},s:{},d:{}}}",
                        ts_quote(variant.serde_name()),
                        ts_quote(style),
                        decoder_fields_spec(&variant.fields, schema, parameters)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{k:\"enum\",r:{},t:{},c:{},o:{},f:{},v:[{variants}]}}",
                ts_quote(repr),
                ts_quote(tag),
                ts_quote(content),
                e.open,
                fallback
            )
        }
        crate::Type::Primitive(_) => "\"pass\"".into(),
    }
}

fn collect_decoder_types(
    reference: &crate::TypeReference,
    schema: &crate::Schema,
    names: &mut BTreeSet<String>,
) {
    for argument in &reference.arguments {
        collect_decoder_types(argument, schema, names);
    }
    let Some(type_def) = output_type(schema, &reference.name) else {
        return;
    };
    if !names.insert(reference.name.clone()) {
        return;
    }
    match type_def {
        crate::Type::Struct(s) => {
            for field in s.fields.iter() {
                collect_decoder_types(&field.type_ref, schema, names);
            }
        }
        crate::Type::Enum(e) => {
            for variant in &e.variants {
                for field in variant.fields.iter() {
                    collect_decoder_types(&field.type_ref, schema, names);
                }
            }
        }
        crate::Type::Primitive(_) => {
            if let Some(fallback) = reference.fallback_once(schema.output_types()) {
                collect_decoder_types(&fallback, schema, names);
            }
        }
    }
}

fn render_decoder_specs(schema: &crate::Schema) -> String {
    let mut names = BTreeSet::new();
    for function in schema.functions() {
        let output = match &function.output_type {
            reflectapi_schema::OutputType::Complete { output_type } => output_type.as_ref(),
            reflectapi_schema::OutputType::Stream { item_type } => Some(item_type),
        };
        for reference in output
            .into_iter()
            .chain(function.error_type.as_ref())
            .chain(function.response_headers.as_ref())
        {
            if reaches_decoded_enum(reference, schema, &mut BTreeSet::new()) {
                collect_decoder_types(reference, schema, &mut names);
            }
        }
    }
    let entries = names
        .into_iter()
        .filter_map(|name| output_type(schema, &name).map(|type_def| (name, type_def)))
        .map(|(name, type_def)| {
            let parameters = type_def
                .parameters()
                .enumerate()
                .map(|(index, param)| (param.name.clone(), index))
                .collect::<HashMap<_, _>>();
            format!(
                "[{}, (args: __DecodeSpec[]) => ({})]",
                ts_quote(&name),
                decoder_type_spec(type_def, schema, &parameters)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!("const __decoder_specs: __DecodeSpecs = Object.fromEntries([\n{entries}\n]);\n")
}

fn render_function(
    function: &Function,
    schema: &crate::Schema,
    implemented_types: &HashMap<String, String>,
) -> Result<String, anyhow::Error> {
    let sig = function_signature(function, schema, implemented_types);
    let name = function.name.replace('-', "_").replace('.', "__");
    let path = format!("{}/{}", function.path, function.name);
    let output_ref = match &function.output_type {
        reflectapi_schema::OutputType::Complete { output_type } => output_type.as_ref(),
        reflectapi_schema::OutputType::Stream { item_type } => Some(item_type),
    };
    let output_decoder = output_ref.and_then(|t| decoder_for_ref(t, schema));
    let error_decoder = function
        .error_type
        .as_ref()
        .and_then(|t| decoder_for_ref(t, schema));
    let headers_decoder = function
        .response_headers
        .as_ref()
        .and_then(|t| decoder_for_ref(t, schema));
    let decoder_args =
        if output_decoder.is_some() || error_decoder.is_some() || headers_decoder.is_some() {
            let mut args = vec![
                output_decoder.unwrap_or_else(|| "undefined".into()),
                error_decoder.unwrap_or_else(|| "undefined".into()),
            ];
            if let Some(headers_decoder) = headers_decoder {
                args.push(headers_decoder);
            }
            format!(
                "{}, {}",
                if sig.response_headers.is_some() {
                    ""
                } else {
                    ", []"
                },
                args.join(", ")
            )
        } else {
            String::new()
        };
    match sig.output {
        FunctionOutput::Single { output_type } => {
            let function_template = templates::FunctionImplementationTemplate {
                name,
                path,
                input_type: sig.input_type,
                input_headers: sig.input_headers,
                output_type,
                error_type: sig.error_type,
                response_headers: sig.response_headers,
                decoder_args,
            };
            Ok(function_template.render())
        }
        FunctionOutput::Stream { item_type } => {
            let function_template = templates::StreamFunctionImplementationTemplate {
                name,
                path,
                input_type: sig.input_type,
                input_headers: sig.input_headers,
                item_type,
                error_type: sig.error_type,
                response_headers: sig.response_headers,
                decoder_args,
            };
            Ok(function_template.render())
        }
    }
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
                alias_template.render()
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
                            let external = flattened_external_enum_ref(
                                &field.type_ref,
                                schema,
                                &mut BTreeSet::new(),
                            );
                            if field.required {
                                // Rust `()` maps to `null` in typescript. This has bad interactions when used with `&`.
                                // Avoid this by turning `null -> {}`.
                                if external {
                                    format!("NullToEmptyObject<__FlattenExternalEnum<{type_ref}>>")
                                } else {
                                    format!("NullToEmptyObject<{type_ref}>")
                                }
                            } else {
                                let nonnullable = type_ref.replace(" | null", "");
                                if external {
                                    format!("Partial<__FlattenExternalEnum<{nonnullable}>>")
                                } else {
                                    format!("Partial<{nonnullable}>")
                                }
                            }
                        })
                        .collect::<Vec<_>>(),
                };
                interface_template.render()
            }
        }
        crate::Type::Enum(enum_def) => {
            let description = doc_to_ts_comments(&enum_def.description, None, 0);

            // An empty enum requires special handling.
            // This is isomorphic to the never type as a value of this type does not exist.
            if enum_def.variants.is_empty() && !enum_def.open {
                return Ok(format!("{description}export type {type_name} = never"));
            }

            let enum_template = templates::Enum {
                name: type_name,
                description,
                open: enum_def.open,
                variants: enum_def
                    .variants
                    .iter()
                    .map(|variant| templates::Variant {
                        name: variant.serde_name().into(),
                        description: doc_to_ts_comments(
                            &variant.description,
                            variant.deprecation().map(|d| d.message()).as_deref(),
                            4,
                        ),
                        deprecation_message: variant.deprecation().map(|d| d.message()),
                        representation: enum_def.representation.clone(),
                        fields: fields_to_ts_fields(&variant.fields, schema, implemented_types),
                        discriminant: variant.discriminant,
                        untagged: variant.untagged,
                    })
                    .collect::<Vec<_>>(),
            };
            enum_template.render()?
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
            alias_template.render()
        }
    })
}

fn flattened_external_enum_ref(
    reference: &crate::TypeReference,
    schema: &crate::Schema,
    visiting: &mut BTreeSet<String>,
) -> bool {
    if matches!(
        reference.name.as_str(),
        "std::option::Option" | "reflectapi::Option" | "std::boxed::Box"
    ) {
        return reference
            .arguments
            .first()
            .is_some_and(|inner| flattened_external_enum_ref(inner, schema, visiting));
    }
    if !visiting.insert(reference.name.clone()) {
        return false;
    }
    let result = match output_type(schema, &reference.name) {
        Some(crate::Type::Enum(enm)) => {
            matches!(enm.representation, crate::Representation::External)
                && enm
                    .variants
                    .iter()
                    .any(|variant| matches!(&variant.fields, crate::Fields::None))
        }
        Some(ty @ crate::Type::Struct(strukt)) if strukt.is_alias() => strukt
            .fields
            .iter()
            .next()
            .map(|field| super::bind_type_parameters(&field.type_ref, ty, reference))
            .is_some_and(|inner| flattened_external_enum_ref(&inner, schema, visiting)),
        Some(crate::Type::Primitive(_)) => reference
            .fallback_once(schema.output_types())
            .is_some_and(|inner| flattened_external_enum_ref(&inner, schema, visiting)),
        _ => false,
    };
    visiting.remove(&reference.name);
    result
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
        description: doc_to_ts_comments(
            &field.description,
            field.deprecation().map(|d| d.message()).as_deref(),
            4,
        ),
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
    format!("{n}{p}")
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
        format!("<{p}>")
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
    format!("{n}{p}")
}

fn type_params_to_ts_name(type_params: std::slice::Iter<'_, crate::TypeParameter>) -> String {
    let p = type_params
        .map(|type_param| type_param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    if p.is_empty() {
        p
    } else {
        format!("<{p}>")
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

    // A `*/` in doc or note text would close the comment early.
    std::iter::once(format!("{padding}/**"))
        .chain(
            doc.split('\n')
                .map(|s| format!("{padding} * {}", s.replace("*/", "*\\/"))),
        )
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

#[cfg(test)]
mod enum_decoder_tests {
    use super::*;

    #[test]
    fn empty_fallback_tag_is_distinct_from_no_fallback() {
        let mut enm = reflectapi_schema::Enum::new("Example".into());
        enm.representation = crate::Representation::Internal { tag: "kind".into() };
        let without_fallback = decoder_type_spec(
            &crate::Type::Enum(enm.clone()),
            &crate::Schema::new(),
            &HashMap::new(),
        );
        assert!(without_fallback.contains("f:null"));

        // A schema can carry an empty wire tag. Keep it distinct from the
        // absence of a serde(other) variant when emitting decoder metadata.
        let mut fallback = reflectapi_schema::Variant::new(String::new());
        fallback.other = true;
        enm.variants.push(fallback);
        let with_fallback = decoder_type_spec(
            &crate::Type::Enum(enm),
            &crate::Schema::new(),
            &HashMap::new(),
        );
        assert!(with_fallback.contains("f:\"\""));
    }
}
