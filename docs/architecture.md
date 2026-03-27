# ReflectAPI Architecture

## 1. Overview

ReflectAPI is a Rust framework for defining web API services and generating 100%-compatible, type-safe clients in multiple languages. The core idea: you write Rust handler functions with strongly-typed inputs and outputs, derive macros capture the type information into a JSON schema, and code generation backends produce idiomatic clients for TypeScript, Rust, Python, and OpenAPI specs.

The system is designed around a multi-stage pipeline:

1. **Derive macros** extract type metadata at compile time into a raw `Schema`.
2. The schema is serialized to JSON (`reflectapi.json`), forming the contract between server and clients.
3. At codegen time, the schema is deserialized, normalized through a pipeline of transformations, and fed to language-specific backends that emit client code.

## 2. Crate Structure

```
reflectapi/                     Workspace root
  reflectapi-schema/            Core schema types, semantic IR, normalization pipeline
  reflectapi-derive/            Proc macros: #[derive(Input, Output, reflectapi::Input, ...)]
  reflectapi/                   Builder, codegen backends, runtime (axum integration, etc.)
  reflectapi-cli/               CLI tool wrapping codegen (clap-based)
  reflectapi-demo/              Demo application with snapshot tests (insta)
  reflectapi-python-runtime/    Python runtime support library
  docs/                         User-facing documentation (mdbook)
```

### reflectapi-schema

The foundational crate. Contains:

- **Raw schema types** (`Schema`, `Function`, `Type`, `Struct`, `Enum`, `Field`, `Variant`, `TypeReference`, `Representation`, `Primitive`, `Typespace`) -- the serializable representation of an API.
- **SymbolId and SymbolKind** -- stable unique identifiers for all schema symbols.
- **ensure_symbol_ids()** -- post-deserialization ID assignment.
- **NormalizationPipeline** -- multi-stage schema transformation (type consolidation, naming, circular dependency resolution).
- **Normalizer** -- converts raw `Schema` into validated `SemanticSchema`.
- **SemanticSchema** -- immutable, fully-resolved intermediate representation.
- **Visitor/VisitMut** -- generic traversal utilities.
- **Substitute/Instantiate** -- generic type parameter substitution.
- **Rename/Glob** -- type renaming and glob-based filtering.

### reflectapi-derive

Procedural macros that implement `#[derive(Input)]` and `#[derive(Output)]`. These parse Rust struct/enum definitions (including all serde attributes) and emit code that registers the type into the `Schema` at build time.

Key source files:
- `lib.rs` -- macro entry points
- `parser.rs` -- serde attribute parsing
- `context.rs` -- code generation context
- `derive.rs` -- the derive implementation
- `tokenizable_schema.rs` -- converts schema types to token streams

### reflectapi

The main user-facing crate. Contains:
- **Builder** -- runtime schema construction and handler registration.
- **Codegen backends** -- `typescript`, `rust`, `python`, `openapi` modules under `src/codegen/`.
- **Runtime integration** -- axum handler adapters, serialization modes (JSON, msgpack).
- **Format utilities** -- post-generation formatting (prettier, rustfmt, ruff).

### reflectapi-cli

A thin CLI wrapper using `clap`. Reads a `reflectapi.json` schema file and invokes the appropriate codegen backend:

```
reflectapi codegen --language typescript --schema reflectapi.json --output ./clients/ts
```

Supports flags for formatting, typechecking, tag filtering, shared modules (Rust), and tracing instrumentation.

### reflectapi-demo

A working demo server with snapshot tests. The `reflectapi.json` file and generated clients (TypeScript, Rust, Python) are checked in and validated by `cargo insta` snapshot tests, ensuring codegen output is stable across changes.

## 3. Schema Types

The raw schema is a JSON-serializable representation of a Rust API. The core types in `reflectapi-schema/src/lib.rs`:

### Schema

Top-level container holding the API definition:

```rust
pub struct Schema {
    pub name: String,
    pub description: String,
    pub functions: Vec<Function>,
    pub input_types: Typespace,   // Types used in request positions
    pub output_types: Typespace,  // Types used in response positions
}
```

The separation into `input_types` and `output_types` allows the same Rust type name to have different shapes in request vs response contexts. The `TypeConsolidationStage` merges these during normalization.

### Function

An API endpoint:

```rust
pub struct Function {
    pub name: String,           // e.g. "users.login"
    pub path: String,           // URL path, e.g. "/api/v1"
    pub input_type: Option<TypeReference>,
    pub input_headers: Option<TypeReference>,
    pub output_type: Option<TypeReference>,
    pub error_type: Option<TypeReference>,
    pub serialization: Vec<SerializationMode>,
    pub readonly: bool,
    pub tags: BTreeSet<String>,
}
```

### Type

A sum type covering all possible type definitions:

```rust
pub enum Type {
    Primitive(Primitive),  // Opaque types (String, i32, Uuid, etc.)
    Struct(Struct),        // Named/unnamed fields, tuple structs
    Enum(Enum),            // Tagged/untagged unions with variants
}
```

### TypeReference

A reference to a type, including generic arguments:

```rust
pub struct TypeReference {
    pub name: String,                   // Fully-qualified name, e.g. "std::vec::Vec"
    pub arguments: Vec<TypeReference>,  // Generic type arguments
}
```

### Representation

Captures serde's enum tagging strategy:

```rust
pub enum Representation {
    External,                              // {"variant": {...}}
    Internal { tag: String },              // {"type": "variant", ...}
    Adjacent { tag: String, content: String }, // {"t": "variant", "c": {...}}
    None,                                  // untagged
}
```

### Fields, Field, Variant

Fields can be `Named`, `Unnamed` (tuple), or `None` (unit). Each `Field` carries:
- `name` / `serde_name` -- the Rust name and the wire name (after serde rename)
- `type_ref` -- the field's type
- `required` -- whether the field must be present
- `flattened` -- whether `#[serde(flatten)]` is applied
- `description` / `deprecation_note` -- documentation metadata

## 4. Semantic IR Pipeline

The pipeline transforms a raw, deserialized `Schema` into a validated, immutable `SemanticSchema` suitable for code generation. This is a multi-phase process.

### Pipeline Overview

```
                                +--------------------------+
  reflectapi.json  ----------> |    JSON Deserialization   |
                                +--------------------------+
                                            |
                                            v
                                +--------------------------+
                                |   ensure_symbol_ids()    |  Assign stable SymbolIds
                                +--------------------------+
                                            |
                                            v
                                +--------------------------+
                                | NormalizationPipeline    |
                                |  1. TypeConsolidation    |  Merge input/output typespaces
                                |  2. NamingResolution     |  Strip module paths, resolve conflicts
                                |  3. CircularDependency   |  Detect cycles via Tarjan's SCC
                                +--------------------------+
                                            |
                                            v
                                +--------------------------+
                                |       Normalizer         |
                                |  Phase 1: Discovery      |  Register all symbols
                                |  Phase 2: Resolution     |  Resolve type references
                                |  Phase 3: Dependencies   |  Build dependency graph
                                |  Phase 4: Validation     |  Semantic checks
                                |  Phase 5: IR Build       |  Construct SemanticSchema
                                +--------------------------+
                                            |
                                            v
                                +--------------------------+
                                |    SemanticSchema        |  Immutable, validated IR
                                +--------------------------+
```

### SymbolId

Every symbol in the schema (types, functions, fields, variants) gets a stable, unique identifier:

```rust
pub struct SymbolId {
    pub kind: SymbolKind,       // Struct, Enum, Endpoint, Field, Variant, Primitive, TypeAlias
    pub path: Vec<String>,      // e.g. ["api", "User"]
    pub disambiguator: u32,     // Distinguishes same-name types across input/output
}
```

The `SymbolId` is the primary key for all lookups throughout the pipeline and in the final `SemanticSchema`. The `disambiguator` field handles the case where input and output typespaces define different types with the same fully-qualified name.

### ensure_symbol_ids()

When a schema is deserialized from JSON, all `SymbolId` fields have their default ("unknown") values. `ensure_symbol_ids()` walks the schema and assigns stable IDs based on canonical names:

1. Pre-registers well-known stdlib types (`String`, `Vec`, `Option`, `HashMap`, etc.)
2. Assigns IDs to functions based on their names.
3. Assigns IDs to types in each typespace, using separate seen-maps so cross-typespace conflicts are detected.
4. When the same FQN appears in both typespaces with different type definitions, the output typespace's copy gets a disambiguated ID (disambiguator = 1).
5. Assigns IDs to struct fields and enum variants as children of their parent's path.

### NormalizationPipeline

The standard pipeline applies three stages in sequence:

**Stage 1: TypeConsolidationStage** -- Merges `input_types` and `output_types` into a single unified `input_types` collection. When a type name appears in both typespaces:
- If the types are identical, the input version is kept.
- If they differ, they're renamed with `input.` / `output.` prefixes, and all type references throughout the schema are rewritten to point to the new names.

**Stage 2: NamingResolutionStage** -- Strips module path prefixes from type names (e.g. `myapp::proto::User` becomes `User`). When stripping creates a conflict (two different types both resolve to `User`), a unique name is generated by incorporating the module prefix (e.g. `ProtoUser` vs `ApiUser`). Common module names like `model` and `proto` are skipped when building the prefix.

**Stage 3: CircularDependencyResolutionStage** -- Detects circular type dependencies using Tarjan's strongly-connected components (SCC) algorithm. Supports multiple resolution strategies:
- `Intelligent` (default) -- self-references use boxing; multi-type cycles use forward declarations.
- `Boxing` -- wrap cyclic references in `Box<T>` (currently a no-op since Rust schemas already encode boxing).
- `ForwardDeclarations`, `OptionalBreaking`, `ReferenceCounted` -- planned strategies (currently stubs).

The cycle detection itself is valuable even when resolution is a no-op, as downstream codegen backends can use this information to emit forward-reference annotations.

### Normalizer

The `Normalizer` orchestrates the full conversion from `Schema` to `SemanticSchema`:

1. **Phase 0**: Calls `ensure_symbol_ids()` and runs `NormalizationPipeline::standard()`.
2. **Phase 1 (Symbol Discovery)**: Walks the schema and registers every type, function, field, and variant in the `SymbolTable`.
3. **Phase 2 (Type Resolution)**: Resolves all `TypeReference` names to `SymbolId` targets, producing `ResolvedTypeReference` values with guaranteed validity.
4. **Phase 3 (Dependency Analysis)**: Builds a dependency graph in the `SymbolTable` and performs topological sorting.
5. **Phase 4 (Semantic Validation)**: Checks for unresolved references, conflicting definitions, and invalid generic parameters.
6. **Phase 5 (IR Construction)**: Builds the final `SemanticSchema` with `BTreeMap`-ordered collections for deterministic output.

### SemanticSchema

The final, immutable output of the pipeline:

```rust
pub struct SemanticSchema {
    pub functions: BTreeMap<SymbolId, SemanticFunction>,
    pub types: BTreeMap<SymbolId, SemanticType>,
    pub symbol_table: SymbolTable,
}
```

Key differences from the raw `Schema`:
- **Immutable** -- no mutation after construction.
- **Fully resolved** -- all type references are `ResolvedTypeReference` with guaranteed valid `SymbolId` targets.
- **Deterministically ordered** -- `BTreeMap`/`BTreeSet` everywhere for stable codegen output.
- **Single typespace** -- input/output distinction has been resolved.
- **Dependency graph** -- the `SymbolTable` contains dependency edges and supports topological sorting.

## 5. Code Generation Backends

All backends live in `reflectapi/src/codegen/`. Each reads the raw `Schema` (the semantic IR pipeline is not yet wired into the codegen path -- see Current Status) and produces language-specific output.

### TypeScript

Uses the `askama` template engine. Key design decisions:

- **Interfaces** for struct types with named fields.
- **Type aliases** for tuple structs and transparent wrappers.
- **Intersection types** (`&`) for `#[serde(flatten)]` -- flattened fields are rendered as separate type references joined with `&`.
- **Discriminated unions** for tagged enums.
- **`NullToEmptyObject<T>`** helper to handle the interaction between Rust `()` (which maps to `null`) and intersection types.
- Output is formatted with `prettier`.

### Rust

Mirrors the source Rust types, re-emitting struct/enum definitions with appropriate `#[serde(...)]` attributes. Key features:

- Preserves `#[serde(flatten)]` on fields that had it in the original.
- Handles shared modules -- types from specified module prefixes are imported rather than regenerated.
- Generates a client module with async functions for each endpoint.
- Output is formatted with `rustfmt`.

### Python

Generates Pydantic v2 models. Key features:

- **BaseModel classes** for structs.
- **Discriminated unions** (`Union[..., Field(discriminator="tag")]`) for internally-tagged enums.
- **RootModel** wrappers for union types and single-field tuple structs.
- Special handling for `#[serde(flatten)]` with internally-tagged enums (see Section 6).
- **Field aliases** for serde-renamed fields.
- **Literal types** for discriminator fields.
- Python reserved words are sanitized in field names with alias mapping.
- Output is formatted with `ruff`.

### OpenAPI

Generates an OpenAPI 3.0 specification (JSON). Key features:

- **`$ref`** for type references, with inline expansion for simple cases.
- **`allOf`** composition for `#[serde(flatten)]` -- the parent object schema and each flattened type schema are combined under `allOf`.
- **`oneOf`** for enum variants (tagged and untagged).
- **`discriminator`** property for internally-tagged enums.
- Rendered via Redoc for documentation.

## 6. Flattened Type Handling

`#[serde(flatten)]` is one of the most complex serde features to handle across languages because it merges fields from one type into another at the wire level. The challenge is especially acute for internally-tagged enums, where the tag field and variant fields are merged flat into the parent struct.

### Wire Format

Given this Rust code:

```rust
#[derive(Serialize, Deserialize)]
struct Request {
    id: String,
    #[serde(flatten)]
    payload: Action,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Action {
    Create { name: String },
    Delete { reason: Option<String> },
}
```

Serde produces this JSON for `Request { id: "1", payload: Action::Create { name: "foo" } }`:

```json
{"id": "1", "type": "Create", "name": "foo"}
```

All fields are merged flat -- there is no nesting. The codegen backends must produce types that match this exact wire format.

### TypeScript

Uses intersection types to compose the parent fields with the flattened type:

```typescript
type Request = {
    id: string;
} & NullToEmptyObject<Action>;
```

The `NullToEmptyObject<T>` wrapper handles the edge case where a flattened type resolves to `null` (Rust `()`), preventing `{...} & null` from collapsing to `never`.

For optional flattened fields, `Partial<T>` is used instead.

### Python (standard flatten)

For flattened structs (not enums), all fields from the flattened type are expanded inline into the parent Pydantic model:

```python
class Request(BaseModel):
    id: str
    # Fields from FlattenedStruct expanded here:
    extra_field: str
```

### Python (internally-tagged enum flatten)

This is the most complex case. When a struct flattens an internally-tagged enum, per-variant models are generated that merge the parent's fields, the tag discriminator, and the variant's fields:

```python
class RequestCreate(BaseModel):
    """'Create' variant of Request"""
    id: str
    type: Literal['Create'] = "Create"
    name: str

class RequestDelete(BaseModel):
    """'Delete' variant of Request"""
    id: str
    type: Literal['Delete'] = "Delete"
    reason: str | None = None

class Request(RootModel):
    root: Annotated[
        Union[
            RequestCreate,
            RequestDelete,
        ],
        Field(discriminator="type"),
    ]
```

Each per-variant model contains all parent struct fields, the `Literal`-typed tag discriminator, and the variant-specific fields. The parent type becomes a `RootModel` with a discriminated union.

The implementation handles several edge cases:
- **Unnamed variants** with a single field: the inner struct's fields are expanded.
- **Box-wrapped variants**: `Box<T>` is unwrapped to get the inner type.
- **Optional flattened fields**: all expanded fields become optional.
- **Multiple flattened structs**: non-enum flattened structs are also expanded into each variant model.

### OpenAPI

Uses `allOf` composition:

```json
{
  "allOf": [
    { "$ref": "#/components/schemas/Action" },
    {
      "type": "object",
      "properties": {
        "id": { "type": "string" }
      },
      "required": ["id"]
    }
  ]
}
```

For enums, `oneOf` is used for the variant schemas, and `discriminator` is set for internally-tagged enums.

### Rust

Preserves the original `#[serde(flatten)]` attribute on the field, since the generated Rust client uses serde directly and the attribute works as-is.

## 7. Current Status and Roadmap

### Complete

- **Raw schema types** and JSON serialization -- stable and well-tested.
- **Derive macros** -- full support for serde attributes including rename, skip, flatten, tag, default.
- **TypeScript codegen** -- stable, with intersection types for flatten.
- **Rust codegen** -- stable, mirrors source types.
- **Python codegen** -- experimental but functional, including the complex flatten+internal-tag case.
- **OpenAPI codegen** -- stable, allOf/oneOf composition.
- **CLI tool** -- feature-complete with tag filtering, formatting, typechecking options.
- **Snapshot testing** -- comprehensive insta-based tests for all codegen outputs.

### In Progress

- **Semantic IR pipeline (related to #96)** -- The `SymbolId`, `ensure_symbol_ids()`, `NormalizationPipeline`, `Normalizer`, and `SemanticSchema` infrastructure is implemented in `reflectapi-schema` but is not yet wired into the codegen backends. The codegen backends currently consume the raw `Schema` directly. The goal is to have all backends consume `SemanticSchema` instead, which provides:
  - Guaranteed resolved references (no dangling type names).
  - Deterministic ordering for stable output.
  - Dependency-aware topological ordering.
  - A single unified typespace.

- **Flattened internally-tagged enum handling (#123)** -- The Python backend has full support. TypeScript handles it via intersection types (which works but is less precise than per-variant types). OpenAPI uses allOf composition. Further improvements may align all backends on a common strategy.

### Known Gaps

- **CircularDependencyResolutionStage** -- Cycle detection works (Tarjan's SCC), but resolution strategies beyond boxing are stubs (`ForwardDeclarations`, `OptionalBreaking`, `ReferenceCounted` are TODO).
- **Codegen consuming SemanticSchema** -- The transition from raw `Schema` to `SemanticSchema` in the codegen path is not yet complete.
- **Python codegen** -- Marked experimental. Some edge cases in generics and complex nested flatten scenarios may not be fully covered.
- **chrono-tz support** -- Recently added (#117), may have edge cases.
