# Feature Flags

Complete reference for ReflectAPI feature flags and build configuration.

## Feature Flags

ReflectAPI uses feature flags to enable specific functionality:

### Core Features

```toml
[dependencies]
reflectapi = { 
    version = "0.15", 
    features = [
        "builder",  # Required for API definition
        "json",     # JSON serialization support
    ] 
}
```

### Web Framework Integration

```toml
[dependencies]
reflectapi = { 
    version = "0.15", 
    features = ["builder", "axum"] 
}
axum = "0.8"
tokio = { version = "1.0", features = ["full"] }
```

### Code Generation

```toml
[dependencies]
reflectapi = { 
    version = "0.15", 
    features = ["builder", "codegen"] 
}
```

### External Type Support

```toml
[dependencies]
reflectapi = { 
    version = "0.15", 
    features = [
        "builder",
        "uuid",        # UUID support
        "chrono",      # Date/time types
        "url",         # URL types
        "rust_decimal" # Decimal types
    ] 
}
uuid = { version = "1.0", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
url = { version = "2.0", features = ["serde"] }
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
reflectapi = { version = "0.15", features = ["builder"] }
serde = { version = "1.0", features = ["derive"] }

# api-server/Cargo.toml  
[dependencies]
api-types = { path = "../api-types" }
reflectapi = { version = "0.15", features = ["axum"] }
axum = "0.8"
```

## Development Dependencies

For development and testing:

```toml
[dev-dependencies]
tokio-test = "0.4"
serde_json = "1.0"
```

## Version Compatibility

| ReflectAPI | Rust | Serde | Axum |
|------------|------|-------|------|
| 0.15.x     | 1.78+ | 1.0   | 0.8  |

## Troubleshooting

### Common Issues

**"feature `X` is required"**: Add the required feature flag to your `Cargo.toml`.

**Compilation errors with derive macros**: Ensure you have both `Input` and `Output` traits in scope.

**CLI not found**: Make sure `~/.cargo/bin` is in your PATH after installing the CLI.

### Getting Help

- Check the [troubleshooting guide](./troubleshooting.md)
- Open an issue on [GitHub](https://github.com/thepartly/reflectapi)
- Read the [API documentation](https://docs.rs/reflectapi)