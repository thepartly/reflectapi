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
    let schema: reflect::Schema = serde_json::from_str(&schema_as_json)
        .context("Failed to parse schema file as JSON into reflect::Schema object")?;
    match language {
        crate::Language::Typescript => {
            typescript::generate(schema, output)?;
        }
    }

    Ok(())
}
