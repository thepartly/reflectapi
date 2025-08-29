# Feature Flags

Reference for `reflectapi` feature flags and build configuration.

## Feature Flags

`reflectapi` uses feature flags to enable specific functionality:

### Core Features

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = [
        "builder",  # Required for API definition
        "json",     # JSON serialization support
    ] 
}
serde = { version = "1.0", features = ["derive"] }
```

### Web Framework Integration

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = ["builder", "axum"] 
}
axum = "0.8"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### Code Generation

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = ["builder", "codegen"] 
}
serde = { version = "1.0", features = ["derive"] }
```

### External Type Support

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = [
        "builder",
        "uuid",        # UUID support
        "chrono",      # Date/time types
        "url",         # URL types
        "rust_decimal", # Decimal types
        "json",        # JSON serialization
    ] 
}
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.7", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
url = { version = "2.5", features = ["serde"] }
rust_decimal = { version = "1.35", features = ["serde"] }
```

## Python Runtime

If you're generating Python clients, you'll need the Python runtime:

```bash
pip install reflectapi-python-runtime
```

Or with `uv`:

```bash
uv add reflectapi-python-runtime
```

## Workspace Setup

For larger projects, you might want to structure your workspace:

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "api-server",
    "api-types", 
    "clients/rust"
]

# api-types/Cargo.toml
[dependencies]
reflectapi = { version = "0.15.5", features = ["builder"] }
serde = { version = "1.0", features = ["derive"] }

# api-server/Cargo.toml  
[dependencies]
api-types = { path = "../api-types" }
reflectapi = { version = "0.15.5", features = ["axum"] }
axum = "0.8"
serde = { version = "1.0", features = ["derive"] }
```

## Development Dependencies

For development and testing:

```toml
[dev-dependencies]
tokio-test = "0.4"
serde_json = "1.0"
```

## Version Compatibility

| `reflectapi` | Rust | Serde | Axum |
|--------------|------|-------|---------| 
| 0.15.5       | 1.82+ | 1.0   | 0.8  |

## Additional Features

### MessagePack Support

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = ["builder", "msgpack"] 
}
rmp-serde = "1.3"
serde = { version = "1.0", features = ["derive"] }
```

### Client Runtime Features

```toml
[dependencies]
reflectapi = { 
    version = "0.15.5", 
    features = ["builder", "rt", "reqwest"] 
}
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
```

## Troubleshooting

### Common Issues

**"feature `X` is required"**: Add the required feature flag to your `Cargo.toml`.

**Compilation errors with derive macros**: Ensure you have both `Input` and `Output` traits in scope, and include standard derives like `Debug`, `Clone`, `serde::Serialize`, and `serde::Deserialize`.

**Missing standard derives**: Always include `Debug`, `Clone`, `serde::Serialize`, and `serde::Deserialize` when deriving `Input` and `Output`.

**Python runtime not found**: Install the Python runtime package:
```bash
uv add reflectapi-python-runtime
# or
pip install reflectapi-python-runtime
```

**CLI not found**: Make sure `~/.cargo/bin` is in your PATH after installing the CLI.

### Getting Help

- Check the [troubleshooting guide](./troubleshooting.md)
- Open an issue on [GitHub](https://github.com/thepartly/reflectapi)
- Read the [API documentation](https://docs.rs/reflectapi)