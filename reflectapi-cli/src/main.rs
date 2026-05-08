use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generates code for typescript, rust, python or openapi from a reflectapi schema
    Codegen {
        /// Path to the source reflect schema
        #[arg(short, long, value_name = "FILE")]
        schema: Option<PathBuf>,

        /// Path to the target directory for the generated code
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Language to generate code for
        #[arg(short, long)]
        language: Language,

        /// Specific to Rust codegen only.
        /// A module which does not need types generated for a client
        /// because that module is a 3rd party or open source crate
        /// which can be used by the client code directly as a dependency.
        /// Multiple modules can be specified.
        #[arg(long, value_delimiter = ',')]
        shared_modules: Option<Vec<String>>,

        #[arg(short, long, value_delimiter = ',')]
        include_tags: Vec<String>,

        #[arg(short, long, value_delimiter = ',')]
        exclude_tags: Vec<String>,

        /// Typecheck the generated code
        #[arg(short, long, default_value = "false")]
        typecheck: bool,

        /// Format the generated code
        #[arg(short, long, default_value = "true")]
        format: bool,

        /// Instrument the generated code with tracing
        #[arg(short = 'I', long, default_value = "false")]
        instrument: bool,

        // Python-specific options
        /// Package name for the generated Python client
        #[arg(long, default_value = "api_client")]
        python_package_name: String,

        /// Generate async client for Python (default: true)
        #[arg(long, default_value = "true")]
        python_async: bool,

        /// Generate sync client for Python (default: false)
        #[arg(long, default_value = "false")]
        python_sync: bool,

        /// Generate testing utilities for Python (default: false)
        #[arg(long, default_value = "false")]
        python_testing: bool,
    },
    /// Documentation subcommands
    #[command(subcommand)]
    Doc(DocSubcommand),
}

#[derive(Subcommand)]
enum DocSubcommand {
    /// Serve documentation for the reflectapi schema
    Open {
        /// Port to serve the docs on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Path to the source reflectapi schema
        #[arg(default_value = "reflectapi.json")]
        path: PathBuf,
    },
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Language {
    Typescript,
    Rust,
    Python,
    Openapi,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doc(doc) => match doc {
            DocSubcommand::Open { port, path } => {
                let mut path = path.canonicalize()?;
                if path.is_dir() {
                    path.push("reflectapi.json");
                }

                let schema: reflectapi::Schema = serde_json::from_reader(std::fs::File::open(
                    &path,
                )?)
                .context("Failed to parse schema file as JSON into reflectapi::Schema object")?;

                let addr = format!("0.0.0.0:{port}");
                eprintln!("Serving {} on http://{addr}", path.display());
                let openapi = reflectapi::codegen::openapi::Spec::from(&schema);
                rouille::start_server(addr, move |request| {
                    rouille::router!(request,
                        (GET) (/) => { rouille::Response::html(include_str!("../redoc.html")) },
                        (GET) (/openapi) => { rouille::Response::json(&openapi) },
                        _ => rouille::Response::empty_404()
                    )
                })
            }
        },
        Commands::Codegen {
            schema,
            output,
            language,
            shared_modules,
            include_tags,
            exclude_tags,
            typecheck,
            format,
            instrument,
            python_package_name,
            python_async,
            python_sync,
            python_testing,
        } => {
            let include_tags = BTreeSet::from_iter(include_tags);
            let exclude_tags = BTreeSet::from_iter(exclude_tags);

            let schema_path = schema.unwrap_or(std::path::PathBuf::from("reflectapi.json"));
            let schema_as_json = std::fs::read_to_string(schema_path.clone())
                .context(format!("Failed to read schema file: {schema_path:?}"))?;
            let schema: reflectapi::Schema = serde_json::from_str(&schema_as_json)
                .context("Failed to parse schema file as JSON into reflectapi::Schema object")?;

            let files: std::collections::BTreeMap<String, String> = match language {
                Language::Typescript => reflectapi::codegen::typescript::generate(
                    schema,
                    reflectapi::codegen::typescript::Config::default()
                        .format(format)
                        .typecheck(typecheck)
                        .include_tags(include_tags)
                        .exclude_tags(exclude_tags),
                )?,
                Language::Rust => {
                    let content = reflectapi::codegen::rust::generate(
                        schema,
                        reflectapi::codegen::rust::Config::default()
                            .format(format)
                            .typecheck(typecheck)
                            .instrument(instrument)
                            .include_tags(include_tags)
                            .exclude_tags(exclude_tags)
                            .shared_modules(
                                shared_modules.unwrap_or_default().into_iter().collect(),
                            ),
                    )?;
                    let mut files = std::collections::BTreeMap::new();
                    files.insert("generated.rs".to_string(), content);
                    files
                }
                Language::Python => {
                    let config = reflectapi::codegen::python::Config {
                        package_name: python_package_name,
                        generate_async: python_async,
                        generate_sync: python_sync,
                        generate_testing: python_testing,
                        format,
                        base_url: None,
                    };
                    reflectapi::codegen::python::generate_files(schema, &config)?
                }
                Language::Openapi => {
                    let content = reflectapi::codegen::openapi::generate(
                        &schema,
                        reflectapi::codegen::openapi::Config::default()
                            .include_tags(include_tags)
                            .exclude_tags(exclude_tags),
                    )?;
                    let mut files = std::collections::BTreeMap::new();
                    files.insert("openapi.json".to_string(), content);
                    files
                }
            };

            if output == Some(std::path::PathBuf::from("-")) {
                // For stdout, output the first/main file
                if let Some(content) = files.values().next() {
                    println!("{content}");
                }
                return Ok(());
            }

            let output_path = output.unwrap_or_else(|| std::path::PathBuf::from("./"));

            // Resolve where each generated file lands.
            //
            // - If the output path looks like a directory (existing dir, or
            //   ends with a separator), every file is placed inside it
            //   under its codegen-assigned filename.
            // - If the output path looks like a file, the codegen file
            //   whose name matches goes there and any siblings land in the
            //   same parent directory under their codegen-assigned names.
            //   This preserves backward compat with existing scripts that
            //   pass `--output .../generated.ts`.
            // - If the output path looks like a file but no codegen file
            //   matches its name, we error rather than create a directory
            //   at that path (which would surprise existing users).
            let looks_like_dir =
                output_path.is_dir() || output_path.to_string_lossy().ends_with('/');

            if looks_like_dir {
                std::fs::create_dir_all(&output_path).context(format!(
                    "Failed to create output directory: {output_path:?}"
                ))?;
                for (filename, content) in &files {
                    write_file(&output_path.join(filename), content)?;
                }
            } else {
                let primary_name = output_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if !files.contains_key(primary_name) {
                    let expected: Vec<&str> = files.keys().map(String::as_str).collect();
                    anyhow::bail!(
                        "output path {output_path:?} looks like a file, but {language:?} \
                         codegen now emits multiple files: {expected:?}. \
                         Pass a directory path (e.g. --output {:?}) instead.",
                        output_path.parent().unwrap_or(std::path::Path::new(".")),
                    );
                }
                let parent = output_path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                std::fs::create_dir_all(&parent)
                    .context(format!("Failed to create output directory: {parent:?}"))?;
                for (filename, content) in &files {
                    let dest = if filename == primary_name {
                        output_path.clone()
                    } else {
                        parent.join(filename)
                    };
                    write_file(&dest, content)?;
                }
            }
            Ok(())
        }
    }
}

fn write_file(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    let mut file =
        std::fs::File::create(path).context(format!("Failed to create file: {path:?}"))?;
    file.write_all(content.as_bytes())
        .context(format!("Failed to write to file: {path:?}"))?;
    Ok(())
}
