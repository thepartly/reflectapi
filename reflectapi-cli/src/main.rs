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
    /// Generates code for typescript
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

        #[arg(short, long, default_value = "false")]
        typecheck: bool,

        #[arg(short, long, default_value = "true")]
        format: bool,
    },
}

#[derive(ValueEnum, Clone)]
enum Language {
    Typescript,
    Rust,
    Openapi,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Codegen {
            schema,
            output,
            language,
            shared_modules,
            include_tags,
            exclude_tags,
            typecheck,
            format,
        } => {
            let include_tags = BTreeSet::from_iter(include_tags);
            let exclude_tags = BTreeSet::from_iter(exclude_tags);

            let schema_path = schema.unwrap_or(std::path::PathBuf::from("reflectapi.json"));
            let schema_as_json = std::fs::read_to_string(schema_path.clone())
                .context(format!("Failed to read schema file: {:?}", schema_path))?;
            let schema: reflectapi::Schema = serde_json::from_str(&schema_as_json)
                .context("Failed to parse schema file as JSON into reflectapi::Schema object")?;

            let (filename, generated_code) = match language {
                Language::Typescript => (
                    "generated.ts",
                    reflectapi::codegen::typescript::generate(
                        schema,
                        &reflectapi::codegen::typescript::Config {
                            format,
                            typecheck,
                            include_tags,
                            exclude_tags,
                        },
                    )?,
                ),
                Language::Rust => (
                    "generated.rs",
                    reflectapi::codegen::rust::generate(
                        schema,
                        &reflectapi::codegen::rust::Config {
                            format,
                            typecheck,
                            shared_modules: BTreeSet::from_iter(shared_modules.unwrap_or_default()),
                            include_tags,
                            exclude_tags,
                        },
                    )?,
                ),
                Language::Openapi => (
                    "openapi.json",
                    reflectapi::codegen::openapi::generate(
                        &schema,
                        &reflectapi::codegen::openapi::Config {
                            tags: Default::default(),
                            include_tags,
                            exclude_tags,
                        },
                    )?,
                ),
            };

            if output == Some(std::path::PathBuf::from("-")) {
                println!("{generated_code}");
                return Ok(());
            }

            let output = output.unwrap_or_else(|| std::path::PathBuf::from("./"));
            let output = output.join(filename);
            let mut file = std::fs::File::create(output.clone())
                .context(format!("Failed to create file: {:?}", output))?;
            file.write(generated_code.as_bytes())
                .context(format!("Failed to write to file: {:?}", output))?;
        }
    }

    Ok(())
}
