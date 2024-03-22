use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
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
    },
}

#[derive(ValueEnum, Clone)]
enum Language {
    Typescript,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Codegen {
            schema,
            output,
            language,
        } => {
            handle_anyhow_result(generate(schema, output, language));
        }
    }
}

fn handle_anyhow_result(result: anyhow::Result<()>) {
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }
}

fn generate(
    schema: Option<std::path::PathBuf>,
    output: Option<std::path::PathBuf>,
    language: crate::Language,
) -> anyhow::Result<()> {
    let schema_path = schema.unwrap_or(std::path::PathBuf::from("reflectapi.json"));
    let schema_as_json = std::fs::read_to_string(schema_path.clone())
        .context(format!("Failed to read schema file: {:?}", schema_path))?;
    let schema: reflect::Schema = serde_json::from_str(&schema_as_json)
        .context("Failed to parse schema file as JSON into reflect::Schema object")?;
    match language {
        crate::Language::Typescript => {
            let generated_code = reflect::codegen::typescript::generate(schema)?;
            let output = output.unwrap_or_else(|| std::path::PathBuf::from("./"));
            let output = output.join("index.ts");
            let mut file = std::fs::File::create(output.clone())
                .context(format!("Failed to create file: {:?}", output))?;
            println!("{}", generated_code);
            file.write(generated_code.as_bytes())
                .context(format!("Failed to write to file: {:?}", output))?;
        }
    }

    Ok(())
}
