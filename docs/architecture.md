# ReflectAPI Architecture

## 1. Overview

ReflectAPI is a Rust framework for defining web API services and generating type-safe clients in multiple languages. Developers write Rust handler functions with typed inputs and outputs. Derive macros capture type metadata into a JSON schema (`reflectapi.json`). Code generation backends produce idiomatic clients for TypeScript, Rust, and Python, plus OpenAPI specifications.

The system is a multi-stage pipeline:

1. **Derive macros** extract type metadata at compile time into a `Schema`.
2. The schema is serialized to JSON (`reflectapi.json`), forming the contract between server and clients.
3. At codegen time, the schema is deserialized, normalized through a pipeline of transformations, and fed to language-specific backends that emit client code.

## 2. Crate Structure

```
reflectapi/                     Workspace root
  reflectapi-schema/            Schema types, semantic IR, normalization pipeline
  reflectapi-derive/            Proc macros: #[derive(Input, Output)]
  reflectapi/                   Builder, codegen backends, runtime (axum, serialization)
  reflectapi-cli/               CLI tool wrapping codegen (clap-based)
  reflectapi-demo/              Demo application with snapshot tests (insta)
  reflectapi-python-runtime/    Python runtime support library (Pydantic helpers, client base classes)
  docs/                         User-facing documentation (mdbook)
```

### reflectapi-schema

The foundational crate. Contains:

- **Schema types** (`Schema`, `Function`, `Type`, `Struct`, `Enum`, `Field`, `Variant`, `TypeReference`, `Representation`, `Primitive`, `Typespace`) -- the serializable representation of an API.
- **SymbolId and SymbolKind** -- stable unique identifiers for all schema symbols.
- **ensure_symbol_ids()** -- post-deserialization ID assignment.
- **NormalizationPipeline** -- multi-stage schema transformation (type consolidation, naming, circular dependency resolution).
- **Normalizer** -- converts `Schema` into validated `SemanticSchema`.
- **SemanticSchema** -- immutable, fully-resolved intermediate representation.
- **Visitor/VisitMut** -- generic traversal utilities.
- **Substitute/Instantiate** -- generic type parameter substitution.
- **Rename/Glob** -- type renaming and glob-based filtering.

### reflectapi-derive

Procedural macros that implement `#[derive(Input)]` and `#[derive(Output)]`. These macros parse Rust struct/enum definitions (including serde attributes) and emit code that populates the `Schema` at compile time.

Key source files:
- `lib.rs` -- macro entry points
- `parser.rs` -- serde attribute parsing
- `context.rs` -- code generation context
- `derive.rs` -- the derive implementation
- `tokenizable_schema.rs` -- converts schema types to token streams

### reflectapi

The main user-facing crate. Contains:
- **Builder** (`src/builder/`) -- runtime schema construction and handler registration.
- **Codegen backends** -- `typescript.rs`, `rust.rs`, `python.rs`, `openapi.rs` in `src/codegen/`.
- **Runtime integration** -- axum handler adapters, serialization modes (JSON, msgpack).
- **Format utilities** (`src/codegen/format.rs`) -- invokes prettier, rustfmt, or ruff on generated output.

### reflectapi-cli

A thin CLI wrapper using `clap`. Reads a `reflectapi.json` schema file and invokes the appropriate codegen backend:

```
reflectapi codegen --language typescript --schema reflectapi.json --output ./clients/ts
```

Flags control output formatting, type checking, tag-based endpoint filtering, shared Rust module imports, and tracing instrumentation.

### reflectapi-demo

A working demo server with snapshot tests. The `reflectapi.json` file and generated clients (TypeScript, Rust, Python) are checked in and validated by `cargo insta` snapshot tests. Each test type produces 5 snapshots: JSON schema, TypeScript, Rust, OpenAPI, and Python.

### reflectapi-python-runtime

Python package providing base classes and utilities for generated Python clients: `AsyncClientBase`, `ClientBase`, `ApiResponse`, `ReflectapiOption`, mock testing utilities, and middleware support.

## 3. Schema Types

The schema is a JSON-serializable representation of a Rust API. The core types live in `reflectapi-schema/src/lib.rs`.

### Schema

Top-level container holding the API definition:

```rust
pub struct Schema {
    pub id: SymbolId,              // Assigned by ensure_symbol_ids()
    pub name: String,
    pub description: String,
    pub functions: Vec<Function>,
    pub input_types: Typespace,    // Types used in request positions
    pub output_types: Typespace,   // Types used in response positions
}
```

Separating `input_types` from `output_types` allows the same Rust type name to carry different shapes in request and response contexts. The `TypeConsolidationStage` merges these during normalization.

### Function

An API endpoint:

```rust
pub struct Function {
    pub id: SymbolId,
    pub name: String,              // e.g. "users.login"
    pub path: String,              // URL path, e.g. "/api/v1"
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

The pipeline transforms a deserialized `Schema` into a validated, immutable `SemanticSchema` suitable for code generation.

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
                                |       Normalizer         |  5-phase conversion to SemanticSchema
                                +--------------------------+
                                            |
                                            v
                                +--------------------------+
                                |    SemanticSchema        |  Immutable, validated IR
                                +--------------------------+
```

### SymbolId

Every symbol in the schema (types, functions, fields, variants) receives a stable, unique identifier:

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

1. Pre-registers well-known stdlib types (`String`, `Vec`, `Option`, `HashMap`, etc.) from `STDLIB_TYPES`.
2. Assigns IDs to functions based on their names.
3. Assigns IDs to types in each typespace, using separate seen-maps so cross-typespace conflicts are detected.
4. When the same FQN appears in both typespaces with different type definitions, the output typespace's copy receives a disambiguated ID (`disambiguator = 1`).
5. Assigns IDs to struct fields and enum variants as children of their parent's path.

### NormalizationPipeline

The standard pipeline applies three stages in sequence:

**Stage 1: TypeConsolidationStage** -- Merges `input_types` and `output_types` into a single unified `input_types` collection. When a type name appears in both typespaces:
- If the types are identical, the input version is kept.
- If they differ, they are renamed with `input.` / `output.` prefixes, and all type references throughout the schema are rewritten to point to the new names.

**Stage 2: NamingResolutionStage** -- Strips module path prefixes from type names (e.g. `myapp::proto::User` becomes `User`). When stripping creates a conflict (two different types both resolve to `User`), a unique name is generated by incorporating the module prefix (e.g. `ProtoUser` vs `ApiUser`).

**Stage 3: CircularDependencyResolutionStage** -- Detects circular type dependencies using Tarjan's strongly-connected components (SCC) algorithm. Supports multiple resolution strategies:
- `Intelligent` (default) -- self-references use boxing; multi-type cycles use forward declarations.
- `Boxing` -- a no-op because Rust schemas already encode `Box<T>` in their type references.
- `ForwardDeclarations`, `OptionalBreaking`, `ReferenceCounted` -- defined in `ResolutionStrategy` but their `apply_*` methods return `Ok(())` without modifying the schema.

Downstream codegen backends query the detected cycles to emit forward-reference annotations regardless of the resolution strategy.

### Normalizer

The `Normalizer` orchestrates the full conversion from `Schema` to `SemanticSchema`:

1. **Phase 0**: Calls `ensure_symbol_ids()` and runs `NormalizationPipeline::standard()`.
2. **Phase 1 (Symbol Discovery)**: Walks the schema and registers every type, function, field, and variant in the `SymbolTable`.
3. **Phase 2 (Type Resolution)**: Resolves all `TypeReference` names to `SymbolId` targets, producing `ResolvedTypeReference` values.
4. **Phase 3 (Dependency Analysis)**: Builds a dependency graph in the `SymbolTable` and performs topological sorting.
5. **Phase 4 (Semantic Validation)**: Checks for semantic errors.
6. **Phase 5 (IR Construction)**: Builds the final `SemanticSchema` with `BTreeMap`-ordered collections for deterministic output.

### SemanticSchema

The output of the pipeline:

```rust
pub struct SemanticSchema {
    pub id: SymbolId,
    pub name: String,
    pub description: String,
    pub functions: BTreeMap<SymbolId, SemanticFunction>,
    pub types: BTreeMap<SymbolId, SemanticType>,
    pub symbol_table: SymbolTable,
}
```

Key properties compared to `Schema`:
- **Fully resolved** -- all type references are `ResolvedTypeReference` with `SymbolId` targets.
- **Deterministically ordered** -- `BTreeMap`/`BTreeSet` everywhere for stable codegen output.
- **Single typespace** -- input/output distinction has been resolved by `TypeConsolidationStage`.
- **Dependency graph** -- the `SymbolTable` contains dependency edges and supports topological sorting.

## 5. Code Generation Backends

All backends live in `reflectapi/src/codegen/`. Each reads the `Schema` directly and produces language-specific output.

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

### Python (experimental)

Generates Pydantic v2 models. Key features:

- **BaseModel classes** for structs.
- **Discriminated unions** (`Union[..., Field(discriminator="tag")]`) for internally-tagged enums.
- **RootModel** wrappers for union types and single-field tuple structs.
- **Per-variant model expansion** for `#[serde(flatten)]` with internally-tagged enums (see Section 6).
- **Field aliases** for serde-renamed fields.
- **Literal types** for discriminator fields.
- Python reserved words are sanitized in field names with alias mapping.
- Output is formatted with `ruff`.

### OpenAPI

Generates an OpenAPI 3.1 specification (JSON). Key features:

- **`$ref`** for type references, with inline expansion for simple cases.
- **`allOf`** composition for `#[serde(flatten)]` -- the parent object schema and each flattened type schema are combined under `allOf`.
- **`oneOf`** for enum variants (tagged and untagged).
- **`const`** values for discriminator tag fields in internally-tagged enum variants.

## 6. Flattened Type Handling

`#[serde(flatten)]` merges fields from one type into another at the wire level. This requires distinct code generation strategies per target language. Internally-tagged enums compound this: the tag field and variant fields merge flat into the parent struct.

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

### Python (struct flatten)

For flattened structs (not enums), all fields from the flattened type are expanded inline into the parent Pydantic model:

```python
class Request(BaseModel):
    id: str
    # Fields from FlattenedStruct expanded here:
    extra_field: str
```

### Python (internally-tagged enum flatten)

When a struct flattens an internally-tagged enum, per-variant models are generated that merge the parent's fields, the tag discriminator, and the variant's fields:

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
        Union[RequestCreate, RequestDelete],
        Field(discriminator="type"),
    ]
```

Each per-variant model contains all parent struct fields, the `Literal`-typed tag discriminator, and the variant-specific fields. The parent type becomes a `RootModel` with a discriminated union.

The implementation handles several edge cases:
- **Unnamed variants** with a single field: the inner struct's fields are expanded.
- **Box-wrapped variants**: `Box<T>` is unwrapped to get the inner type.
- **Optional flattened fields**: all expanded fields become optional.
- **Multiple flattened structs**: non-enum flattened structs are also expanded into each variant model.

For other enum representations (external, adjacent, untagged), the flattened enum is emitted as a regular typed field. This preserves data but does not match the flat wire format that serde produces.

### OpenAPI

Uses `allOf` composition:

```json
{
  "allOf": [
    { "$ref": "#/components/schemas/Action" },
    {
      "type": "object",
      "properties": { "id": { "type": "string" } },
      "required": ["id"]
    }
  ]
}
```

For enums, `oneOf` is used for the variant schemas with `const` values on tag fields.

### Rust

Preserves the original `#[serde(flatten)]` attribute on the field, since the generated Rust client uses serde directly.

## 7. Limitations and Design Gaps

### Codegen backends consume `Schema` directly

The `SymbolId`, `ensure_symbol_ids()`, `NormalizationPipeline`, `Normalizer`, and `SemanticSchema` infrastructure exists in `reflectapi-schema` but the codegen backends in `reflectapi/src/codegen/` consume the `Schema` directly. Migrating backends to consume `SemanticSchema` would provide guaranteed resolved references, deterministic ordering, dependency-aware topological ordering, and a single unified typespace.

### Flattened enum handling varies by representation

The Python backend generates per-variant models for flattened internally-tagged enums (wire-compatible). For externally-tagged, adjacently-tagged, and untagged enums, the flattened field is emitted as a regular nested field, which does not match the flat wire format. TypeScript uses intersection types for all representations. OpenAPI uses `allOf` composition. The backends do not share a common strategy.

### CircularDependencyResolutionStage

Cycle detection uses Tarjan's SCC algorithm. The `Boxing` strategy is a no-op because Rust schemas already encode `Box<T>`. The `ForwardDeclarations`, `OptionalBreaking`, and `ReferenceCounted` strategies are defined in `ResolutionStrategy` but their `apply_*` methods return `Ok(())` without modifying the schema.

### Python codegen coverage

Marked experimental. Generics and complex nested flatten scenarios have incomplete coverage.
