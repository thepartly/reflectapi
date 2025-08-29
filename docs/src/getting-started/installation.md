# Installation

Get `reflectapi` up and running in minutes.

## Basic Setup


cargo add reflectapi --features builder



```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder"] }
serde = { version = "1.0", features = ["derive"] }
```

## CLI Tool

Install the CLI tool to generate client libraries:

```bash
cargo install reflectapi-cli
```

## Common Integrations

### With Axum Web Framework

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder", "axum"] }
axum = "0.8"
tokio = { version = "1.0", features = ["full"] }
```

### For Client Generation

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder", "codegen"] }
```

## Next Steps

- **New users**: Follow the [Quick Start](./quick-start.md) guide
- **Need more options?**: See [Advanced Installation](../reference/installation.md) for feature flags, workspace setup, and troubleshooting