# Introduction


`reflectapi` is a Rust library for code-first web service API declaration and client generation. It allows 
you to define your API using native Rust types and automatically generate type-safe client libraries for 
TypeScript, Python, and Rust.

## Why `reflectapi`?

`reflectapi`:
- Takes a **Code-first approach**: API definition lives in your Rust code
- Features **client generation**: Generate client SDKs in multiple languages
- Client side **Type inference**: Propagate type information directly to clients from server


## Core Philosophy

`reflectapi` follows a simple principle: **define once, generate everything**. By leveraging Rust's powerful type system and derive macros, you can:

1. Define your API types using standard Rust structs and enums
2. Add derive macros to make them API-compatible
3. Use the builder pattern to define endpoints
4. Generate clients for any supported language


## Ready to Start?

Head over to [Quick Start](./getting-started/quick-start.md) to build your first API with `reflectapi`!