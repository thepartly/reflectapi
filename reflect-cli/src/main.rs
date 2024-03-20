mod codegen;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
            handle_anyhow_result(codegen::generate(schema, output, language));
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
