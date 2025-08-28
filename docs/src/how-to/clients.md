# Generating & Using Clients

Learn how to generate and use type-safe client libraries from your `reflectapi` definition.

## Quick Start

### 1. Generate Schema

First, your Rust application needs to export a schema. Add this to your `main.rs`:

```rust,ignore
use reflectapi::{Builder, Input, Output};

// Your API definitions...
let builder = Builder::new().name("My API");
let (schema, _) = builder.build()?;

// Save schema to file
let schema_json = serde_json::to_string_pretty(&schema)?;
std::fs::write("reflectapi.json", schema_json)?;
```

### 2. Generate Clients

Use the reflectapi CLI to generate clients:

```bash
# Install the CLI
cargo install reflectapi-cli

# Generate TypeScript client
reflectapi codegen --language typescript --schema reflectapi.json --output clients/typescript

# Generate Python client
reflectapi codegen --language python --schema reflectapi.json --output clients/python

# Generate Rust client
reflectapi codegen --language rust --schema reflectapi.json --output clients/rust
```

### 3. Use the Generated Client

#### TypeScript
```typescript
import { client } from './clients/typescript/generated';

const api = client('https://api.example.com');

// Type-safe API calls
const user = await api.users.get({ id: 123 });
const newUser = await api.users.create({ 
  name: 'Alice', 
  email: 'alice@example.com' 
});
```

#### Python
```python
from clients.python.generated import AsyncClient

async def main():
    async with AsyncClient(base_url='https://api.example.com') as client:
        # Type-safe API calls with pydantic models
        user = await client.users_get(id=123)
        new_user = await client.users_create(
            name='Alice',
            email='alice@example.com'
        )
```

#### Rust
```rust,ignore
use clients::rust::generated::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Compile-time type safety
    let user = client.users_get(123).await?;
    let new_user = client.users_create(CreateUserRequest {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    }).await?;
    
    Ok(())
}
```

## Automated Client Generation

Your application must run to generate the schema file. Here are the most practical approaches:

### Option 1: Simple Shell Script (Recommended)

Create a build script that handles the entire workflow:

```bash
#!/bin/bash
# build-and-codegen.sh

echo "Building project..."
cargo build --release

echo "Generating schema..."
timeout 2s cargo run --release || true

if [ ! -f "reflectapi.json" ]; then
    echo "Error: Schema not generated"
    exit 1
fi

echo "Generating clients..."
# Using cargo run if reflectapi not installed globally
REFLECTAPI_BIN="cargo run --manifest-path ../reflectapi/Cargo.toml --bin reflectapi --"

# Or if installed: reflectapi
if command -v reflectapi &> /dev/null; then
    REFLECTAPI_BIN="reflectapi"
fi

$REFLECTAPI_BIN codegen --language typescript --schema reflectapi.json --output clients/typescript
$REFLECTAPI_BIN codegen --language python --schema reflectapi.json --output clients/python

echo "Client generation complete"
```

Usage:
```bash
chmod +x build-and-codegen.sh
./build-and-codegen.sh
```

**Pros:** Simple, transparent, easy to customize  
**Cons:** Unix/Linux only

### Option 2: Built-in CLI Command

Add client generation as a subcommand to your application:

```toml
# Cargo.toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
```

```rust,ignore
use clap::{Parser, Subcommand};
use reflectapi::{Builder, Empty};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate API clients
    Generate {
        #[arg(short, long, default_value = "typescript,python")]
        languages: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let builder: Builder<Empty> = Builder::new().name("My API");
    let (schema, routers) = builder.build()?;
    
    // Always export schema
    let schema_json = serde_json::to_string_pretty(&schema)?;
    std::fs::write("reflectapi.json", schema_json)?;
    
    match cli.command {
        Some(Commands::Generate { languages }) => {
            for lang in languages.split(',') {
                match lang.trim() {
                    "typescript" => {
                        let code = reflectapi::codegen::typescript::generate(
                            schema.clone(), 
                            &reflectapi::codegen::typescript::Config::default()
                        )?;
                        std::fs::create_dir_all("clients/typescript")?;
                        std::fs::write("clients/typescript/generated.ts", code)?;
                        println!("Generated TypeScript client");
                    },
                    "python" => {
                        let code = reflectapi::codegen::python::generate(
                            schema.clone(),
                            &reflectapi::codegen::python::Config::default()
                        )?;
                        std::fs::create_dir_all("clients/python")?;
                        std::fs::write("clients/python/generated.py", code)?;
                        println!("Generated Python client");
                    },
                    _ => eprintln!("Unknown language: {}", lang),
                }
            }
            return Ok(());
        },
        None => {
            // Start server normally
        }
    }
    
    // Server startup code...
    Ok(())
}
```

Usage:
```bash
# Generate clients
cargo run -- generate --languages typescript,python

# Start server
cargo run
```

**Pros:** Cross-platform, integrated with your application  
**Cons:** Requires code changes, adds dependencies

### Option 3: Cargo xtask

For cross-platform automation, use the cargo-xtask pattern:

```toml
# Cargo.toml
[workspace]
members = [".", "xtask"]
```

```toml
# .cargo/config.toml
[alias]
xtask = "run --package xtask --"
```

```rust
// xtask/src/main.rs
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build application
    Command::new("cargo")
        .args(["build", "--release"])
        .status()?;
    
    // Run to generate schema
    let mut child = Command::new("target/release/my-api")
        .spawn()?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    child.kill()?;
    
    // Generate clients
    Command::new("reflectapi")
        .args(["codegen", "--language", "typescript", 
               "--schema", "reflectapi.json", 
               "--output", "clients/typescript"])
        .status()?;
    
    println!("Done");
    Ok(())
}
```

Usage:
```bash
cargo xtask
```

**Pros:** Cross-platform, Rust-native  
**Cons:** More complex setup

### Option 4: Runtime Generation

Generate clients when your application starts:

```rust,ignore
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = my_api::builder();
    let (schema, routers) = builder.build()?;
    
    // Generate TypeScript client on startup
    if std::env::var("GENERATE_CLIENTS").is_ok() {
        let ts_code = reflectapi::codegen::typescript::generate(
            schema.clone(),
            &reflectapi::codegen::typescript::Config::default()
        )?;
        std::fs::create_dir_all("clients/typescript")?;
        std::fs::write("clients/typescript/generated.ts", ts_code)?;
    }
    
    // Start server...
    Ok(())
}
```

Usage:
```bash
GENERATE_CLIENTS=1 cargo run
```

**Pros:** Always up-to-date  
**Cons:** Slower startup

## Comparison

| Approach | Complexity | Cross-Platform | Use Case |
|----------|------------|----------------|----------|
| Shell Script | Low | No | Unix/Linux development |
| Built-in CLI | Medium | Yes | Integrated tooling |
| Cargo xtask | Medium | Yes | CI/CD pipelines |
| Runtime | Low | Yes | Development/prototyping |

## Best Practices

1. **Version Control**: Commit your `reflectapi.json` schema for reproducible builds
2. **CI/CD**: Use shell scripts or cargo-xtask for automated client generation
3. **Package Distribution**: Publish generated clients to package registries (npm, PyPI, crates.io)
4. **Testing**: Include integration tests that verify client compatibility

## Next Steps

- See language-specific guides for [TypeScript](../clients/typescript.md), [Python](../clients/python.md), and [Rust](../clients/rust.md)
- Learn about [Custom Types](./custom-types.md) for advanced type handling
- Explore [Client Comparison](../clients/README.md) for feature differences