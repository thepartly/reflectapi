# Architecture

## Overview

ReflectAPI has three layers:

1. Rust types and handler functions define the API surface.
2. Reflection builds a `Schema`, which is the interchange format between the server side and code generators.
3. Codegen backends transform that schema into language-specific clients or an OpenAPI document.

The workspace is split accordingly:

- `reflectapi-schema`: raw schema types and raw-schema transforms
- `reflectapi-schema-codegen`: compiler-owned IDs, normalization pipeline, semantic IR
- `reflectapi-derive`: `#[derive(Input, Output)]` macros
- `reflectapi`: reflection traits, builder, runtime integrations, codegen backends
- `reflectapi-cli`: CLI wrapper around codegen
- `reflectapi-demo`: snapshot and integration tests
- `reflectapi-python-runtime`: runtime support for generated Python clients

## Reflection Model

Reflection starts from the `Input` and `Output` traits in `reflectapi/src/traits.rs`. Derived implementations and hand-written impls register types into a `Typespace` and return `TypeReference`s that point at those definitions.

The top-level `Schema` in `reflectapi-schema/src/lib.rs` contains:

- `functions`: endpoint definitions
- `input_types`: types seen in request positions
- `output_types`: types seen in response positions

Input and output types stay separate at schema-construction time so the same Rust name can have different request and response shapes. Some backends later consolidate them into a single naming domain.

## Schema and IDs

`SymbolId` and `SymbolKind` live in `reflectapi-schema-codegen/src/symbol.rs`. They are compiler identifiers, not part of the stable JSON contract.

Key points:

- raw `Schema`, `Function`, and type/member definitions do not store symbol IDs
- `build_schema_ids()` in `reflectapi-schema-codegen/src/ids.rs` assigns IDs in a compiler-owned side table
- the schema root now uses `SymbolKind::Schema`
- the schema root path includes the `__schema__` sentinel to avoid colliding with a user-defined type of the same name

That keeps `reflectapi.json` wire-focused while normalization and semantic analysis still get stable identities.

## Type Metadata

Every reflected type is one of:

- `Primitive`
- `Struct`
- `Enum`

`Primitive.fallback` lets a backend substitute a simpler representation when it does not natively model the original Rust type. Examples in the current codebase include pointer-like wrappers falling back to `T`, and ordered collections falling back to unordered equivalents or vectors.

Language-specific metadata is carried by `LanguageSpecificTypeCodegenConfig` in `reflectapi-schema/src/codegen.rs`:

- Rust metadata is serialized when present, for example extra derives on generated Rust types.
- Python type mappings are backend-local in `reflectapi/src/codegen/python.rs`, not schema annotations.

## Normalization

Normalization lives in `reflectapi-schema-codegen/src/normalize.rs`.

There are two parts:

1. A mutable normalization pipeline over raw `Schema`
2. A `Normalizer` that converts the resulting schema into `SemanticSchema`

The configurable pipeline is built with `PipelineBuilder`. The convenience constructors are:

- `NormalizationPipeline::standard()`
  Runs type consolidation, naming resolution, and circular dependency resolution.
- `NormalizationPipeline::for_codegen()`
  Skips consolidation and naming, and only runs circular dependency resolution.

After the pipeline runs, `Normalizer` performs:

1. symbol discovery
2. type resolution
3. dependency analysis
4. semantic validation
5. semantic IR construction

`SemanticSchema` provides resolved, deterministic views of functions and types and is defined in `reflectapi-schema-codegen/src/semantic.rs`.

## Backend Behavior

Backends do not all consume the schema in the same way.

### TypeScript

The TypeScript backend in `reflectapi/src/codegen/typescript.rs` consolidates raw schema types and renders directly from the raw schema.

### Rust

The Rust backend in `reflectapi/src/codegen/rust.rs` also works primarily from the raw schema after consolidation.

### Python

The Python backend in `reflectapi/src/codegen/python.rs` uses both representations:

1. `schema.consolidate_types()` runs first
2. `validate_type_references()` checks raw references
3. `Normalizer::normalize_with_pipeline(...)` builds `SemanticSchema` using a pipeline that skips consolidation and naming
4. rendering uses semantic ordering and symbol information, while still consulting raw schema details where the backend needs original field/type shapes

Python-specific type support is driven by backend-local mappings keyed by canonical Rust type name. Those mappings are static codegen knowledge, not part of the shared schema contract.

### OpenAPI

The OpenAPI backend in `reflectapi/src/codegen/openapi.rs` walks the raw schema directly.

## Runtime-Specific Types

ReflectAPI includes special API-facing types whose semantics matter to codegen:

- `reflectapi::Option<T>`: three-state optional value for PATCH-like APIs
- `reflectapi::Empty`: explicit empty request/response body type
- `reflectapi::Infallible`: explicit “no error payload” type

The Python backend treats these as runtime-provided abstractions rather than generated models.

## Testing and Validation

`reflectapi-demo` is the main regression suite.

The snapshot harness in `reflectapi-demo/src/tests/assert.rs` generates five artifacts per test:

- raw schema JSON
- TypeScript client output
- Rust client output
- OpenAPI output
- Python client output

The workspace also contains compile-pass and compile-fail tests driven by `trybuild`.

This architecture chapter is intended to describe the code paths that exist in the repository, not an aspirational future design.
