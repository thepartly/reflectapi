# Client Generation

`reflectapi` can generate client code from a reflected schema JSON file.

## Supported Outputs

| Output | Status | Notes |
|--------|--------|-------|
| TypeScript | Stable | Single generated file |
| Rust | Stable | Single generated file |
| Python | Experimental | Package-style output with `__init__.py` and `generated.py` |

OpenAPI generation is also supported by the CLI, but it is documented separately as an API description format rather than a client library.

## Workflow

1. Define your API server using `reflectapi` derives and the builder API.
2. Write the schema JSON from your Rust application.
3. Run `reflectapi codegen` for the target language.
4. Commit or consume the generated client code from your application.

The CLI defaults to `reflectapi.json` if `--schema` is omitted. The demo project uses that filename. If your application writes a different filename such as `reflectapi-schema.json`, pass that path explicitly.

```bash
# Create output directories first. TypeScript and Rust write a single file
# unless the output path already exists as a directory or ends with a slash.
mkdir -p clients/typescript clients/python clients/rust

# Generate TypeScript client -> clients/typescript/generated.ts
cargo run --bin reflectapi -- codegen \
  --language typescript \
  --schema reflectapi.json \
  --output clients/typescript/

# Generate Python client -> clients/python/__init__.py and generated.py
cargo run --bin reflectapi -- codegen \
  --language python \
  --schema reflectapi.json \
  --output clients/python/ \
  --python-sync

# Generate Rust client -> clients/rust/generated.rs
cargo run --bin reflectapi -- codegen \
  --language rust \
  --schema reflectapi.json \
  --output clients/rust/
```

If you installed the CLI separately, replace `cargo run --bin reflectapi --` with `reflectapi`.

## Output Shape

The generators do not all emit the same file layout:

| Output | Files written by the generator |
|--------|--------------------------------|
| TypeScript | `generated.ts` |
| Rust | `generated.rs` |
| Python | `__init__.py`, `generated.py` |

The demo repository includes extra project scaffolding around some generated clients, but that scaffolding is not produced by `reflectapi codegen` itself.

## Language Behavior

### TypeScript

- Uses generated TypeScript types and function wrappers.
- Uses a `fetch`-based default client implementation.
- Parses JSON responses, but does not generate runtime schema validators today.
- Supports custom client implementations via the generated client interface.

### Python

- Generates Pydantic-based models and client code.
- Generates an async client by default.
- Adds a sync client only when `--python-sync` is passed.
- Uses `reflectapi_runtime` for client base classes and runtime helpers.

### Rust

- Generates typed async client methods.
- Integrates with `reflectapi::rt::Client`.
- Supports optional tracing instrumentation through `--instrument`.
- Generates serde-compatible types and request helpers for JSON-based transport.

## Shared Characteristics

The generated clients all aim to provide:

- Types derived from the Rust-reflected schema
- Function wrappers with generated documentation
- Structured handling of application errors versus transport/protocol failures
- Good IDE support through generated type information

They do not currently all provide the same runtime validation guarantees or the same runtime transport abstractions, so those details should be considered language-specific.
