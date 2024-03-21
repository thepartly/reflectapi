use std::io::Write;

use anyhow::Context;

mod typescript;

pub fn generate(
    schema: Option<std::path::PathBuf>,
    output: Option<std::path::PathBuf>,
    language: crate::Language,
) -> anyhow::Result<()> {
    let schema_path = schema.unwrap_or(std::path::PathBuf::from("reflectapi.json"));
    let schema_as_json = std::fs::read_to_string(schema_path.clone())
        .context(format!("Failed to read schema file: {:?}", schema_path))?;
    let schema: reflect::EndpointSchema = serde_json::from_str(&schema_as_json)
        .context("Failed to parse schema file as JSON into reflect::Schema object")?;
    match language {
        crate::Language::Typescript => {
            let generated_code = typescript::generate(schema)?;
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
