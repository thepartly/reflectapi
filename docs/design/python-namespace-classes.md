# Design: Python Namespace Classes for Module-Structured Types

## Problem Statement

Rust types like `myapp::orders::v2::OrderStatus` currently produce Python class names
like `MyappOrdersV2OrderStatus` via the `improve_class_name` function, which
concatenates all `::` segments into PascalCase. This is unreadable and produces poor
developer experience. Meanwhile, the TypeScript codegen already emits nested
`namespace` blocks mirroring the Rust module structure, giving users
`myapp.orders.v2.OrderStatus`.

## Goal

Emit Python namespace classes that mirror the Rust module hierarchy:

```python
class orders:
    class v2:
        class OrderStatus(BaseModel):
            ...
```

Users write `orders.v2.OrderStatus` instead of `MyappOrdersV2OrderStatus`.

---

## Current Architecture (Key Code Paths)

### 1. `improve_class_name` (line ~3786)

The single bottleneck where Rust qualified names become Python class names.
For `myapp::orders::v2::OrderStatus` it splits on `::`, PascalCases each segment,
and concatenates: `MyappOrdersV2OrderStatus`.

### 2. `type_ref_to_python_type` (line ~4167)

Resolves a `TypeReference` to a Python type string. For user-defined types (step 3,
line ~4259), it calls `improve_class_name(&type_ref.name)` to produce the class name
used in annotations. This is where `MyappOrdersV2OrderStatus` appears in field type
annotations.

### 3. `generate` function (line ~1095)

The main orchestrator. Key flow:
1. `schema.consolidate_types()` returns `Vec<String>` of all qualified type names.
2. For each type, calls `improve_class_name(type_def.name())` to get the Python name.
3. `rendered_types: BTreeMap<String, String>` maps Python class name -> rendered code.
4. Types are emitted top-level, then `model_rebuild()` calls, then factory classes.

### 4. `rendered_types: BTreeMap<String, String>` (line ~1248)

Maps the flat Python class name (e.g., `MyappOrdersV2OrderStatus`) to its rendered
class definition string. Used for `model_rebuild()` calls and the testing module's
type list.

### 5. TypeScript precedent (`modules_from_rendered_types`, typescript.rs:800)

TypeScript already does exactly what we want: it splits qualified names on `::`, pops
the last segment as the type name, and walks the remaining segments to build a tree of
`Module { name, types, submodules }`. Rendering emits nested `export namespace` blocks.
Type references join `::` segments with `.` (typescript.rs:988).

---

## Design

### A. Namespace Extraction: `qualified_name_to_python`

#### Core function signature

```rust
/// Split a Rust qualified name into (namespace_path, leaf_name).
///
/// After stripping `strip_prefix` (if it matches), the remaining `::` segments
/// become the namespace path, and the final segment is the leaf type name.
fn qualified_name_to_python(
    name: &str,
    strip_prefix: &str,
) -> (Vec<String>, String) {
    let working_name = if !strip_prefix.is_empty() {
        let prefix_with_sep = if strip_prefix.ends_with("::") {
            strip_prefix.to_string()
        } else {
            format!("{}::", strip_prefix)
        };
        name.strip_prefix(&prefix_with_sep).unwrap_or(name)
    } else {
        name
    };

    let parts: Vec<&str> = working_name.split("::").collect();
    if parts.len() <= 1 {
        // No module path -- top-level type
        return (vec![], parts[0].to_string());
    }

    let leaf = parts.last().unwrap().to_string();
    let namespace: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|s| s.to_string())  // keep original casing -- these become Python identifiers
        .collect();

    (namespace, leaf)
}
```

#### How `strip_prefix` is determined

The demo app calls `builder.rename_types("reflectapi_demo::", "myapi::")`, which
transforms `reflectapi_demo::model::Pet` to `myapp::model::Pet`. After
`consolidate_types`, the qualified names already have the user's chosen prefix.

The **common prefix** is the longest shared `::` prefix across all user-defined types.
For the demo, that's `myapi::`. We should compute this automatically:

```rust
fn compute_common_prefix(type_names: &[String]) -> String {
    // Filter to only user-defined types (exclude std::, reflectapi::, chrono::, etc.)
    let user_types: Vec<&str> = type_names.iter()
        .map(|s| s.as_str())
        .filter(|n| !is_implemented_or_runtime_type(n))
        .collect();

    if user_types.is_empty() {
        return String::new();
    }

    // Split each into :: segments, find common prefix segments
    let first_parts: Vec<&str> = user_types[0].split("::").collect();
    let mut common_len = first_parts.len().saturating_sub(1); // never include last (the type name itself)

    for name in &user_types[1..] {
        let parts: Vec<&str> = name.split("::").collect();
        let max = parts.len().saturating_sub(1);
        common_len = common_len.min(max);
        for i in 0..common_len {
            if parts[i] != first_parts[i] {
                common_len = i;
                break;
            }
        }
    }

    if common_len == 0 {
        return String::new();
    }

    first_parts[..common_len].join("::")
}
```

The `Config` should also accept an optional explicit `strip_prefix` override, since
the auto-detection can be surprising when a schema has types from many different crate
roots:

```rust
pub struct Config {
    // ... existing fields ...
    /// Prefix to strip from Rust module paths when generating Python namespaces.
    /// If None, the common prefix is auto-detected.
    /// If Some(""), no prefix is stripped (all module segments become namespaces).
    pub namespace_strip_prefix: Option<String>,
}
```

#### Examples

| Rust qualified name | strip_prefix | namespace_path | leaf_name |
|---|---|---|---|
| `myapp::orders::v2::OrderStatus` | `myapp` | `["orders", "v2"]` | `OrderStatus` |
| `myapp::model::Pet` | `myapp` | `["model"]` | `Pet` |
| `myapp::model::input::Pet` | `myapp` | `["model", "input"]` | `Pet` |
| `myapp::proto::PetsListRequest` | `myapp` | `["proto"]` | `PetsListRequest` |
| `OrderStatus` | `""` | `[]` | `OrderStatus` |
| `std::string::String` | `myapp` | (handled by implemented_types, never reaches this) | - |

### B. Type Name Mapping: Replace `improve_class_name`

#### New data structure: `TypeNameMapping`

```rust
/// Pre-computed mapping from Rust qualified names to Python namespace + leaf names.
struct TypeNameMapping {
    /// For each original Rust type name, the Python namespace path and leaf class name.
    /// E.g., "myapp::orders::v2::OrderStatus" -> (["orders", "v2"], "OrderStatus")
    entries: BTreeMap<String, (Vec<String>, String)>,

    /// The dotted Python reference string for each Rust type name.
    /// E.g., "myapp::orders::v2::OrderStatus" -> "orders.v2.OrderStatus"
    python_refs: BTreeMap<String, String>,
}

impl TypeNameMapping {
    fn new(
        all_type_names: &[String],
        implemented_types: &BTreeMap<String, String>,
        strip_prefix: &str,
    ) -> Self {
        let mut entries = BTreeMap::new();
        let mut python_refs = BTreeMap::new();

        for name in all_type_names {
            if implemented_types.contains_key(name) {
                continue;
            }

            let (ns_path, leaf) = qualified_name_to_python(name, strip_prefix);
            let dotted = if ns_path.is_empty() {
                leaf.clone()
            } else {
                format!("{}.{}", ns_path.join("."), leaf)
            };

            entries.insert(name.clone(), (ns_path, leaf));
            python_refs.insert(name.clone(), dotted);
        }

        Self { entries, python_refs }
    }

    /// Get the Python reference string for use in type annotations.
    /// Returns the dotted path like "orders.v2.OrderStatus".
    fn python_ref(&self, rust_name: &str) -> Option<&str> {
        self.python_refs.get(rust_name).map(|s| s.as_str())
    }

    /// Get the leaf class name (the name used in the class definition itself).
    fn leaf_name(&self, rust_name: &str) -> Option<&str> {
        self.entries.get(rust_name).map(|(_, leaf)| leaf.as_str())
    }

    /// Get the namespace path segments.
    fn namespace_path(&self, rust_name: &str) -> Option<&[String]> {
        self.entries.get(rust_name).map(|(ns, _)| ns.as_slice())
    }
}
```

#### Replace `improve_class_name` calls

`improve_class_name` is called in ~15 places. It must be replaced by lookups into
`TypeNameMapping`:

- **In class definitions** (the `class Foo(BaseModel):` line): use `leaf_name()`.
  The class is *defined* inside its namespace wrapper, so only the leaf is needed.

- **In type annotations** (field types, union members, function params/returns): use
  `python_ref()`. This gives the fully-qualified dotted path like
  `orders.v2.OrderStatus`.

- **In variant class names** (e.g., `MyapiModelBehaviorAggressiveVariant`): these are
  derived names. Since the enum itself has a namespace + leaf, the variant class should
  live in the same namespace with name `{EnumLeaf}{VariantName}Variant`. E.g.,
  `BehaviorAggressiveVariant` inside namespace `model`.

Concretely, every call site for `improve_class_name` must receive the `TypeNameMapping`
(or at least the original Rust qualified name) and choose between `leaf_name` and
`python_ref` depending on context.

### C. Output Structure: Namespace Module Tree

#### Data structure (mirroring TypeScript)

```rust
/// A node in the Python namespace tree.
struct NamespaceNode {
    name: String,
    /// Rendered type definitions (class bodies) that belong directly to this namespace.
    types: Vec<String>,
    /// Child namespaces.
    children: BTreeMap<String, NamespaceNode>,
}
```

#### Building the tree

Identical to `modules_from_rendered_types` in TypeScript/Rust codegens:

```rust
fn build_namespace_tree(
    type_names: &[String],        // original Rust qualified names
    rendered_types: &mut BTreeMap<String, String>,  // leaf_name -> rendered code
    mapping: &TypeNameMapping,
) -> NamespaceNode {
    let mut root = NamespaceNode {
        name: String::new(),
        types: vec![],
        children: BTreeMap::new(),
    };

    for rust_name in type_names {
        if let Some((ns_path, _leaf)) = mapping.entries.get(rust_name) {
            let mut node = &mut root;
            for segment in ns_path {
                node = node.children
                    .entry(segment.clone())
                    .or_insert_with(|| NamespaceNode {
                        name: segment.clone(),
                        types: vec![],
                        children: BTreeMap::new(),
                    });
            }
            // The key in rendered_types needs to be whatever we used when inserting.
            // We'll use the dotted python ref as the key.
            let python_ref = mapping.python_ref(rust_name).unwrap();
            if let Some(rendered) = rendered_types.remove(python_ref) {
                node.types.push(rendered);
            }
        }
    }

    root
}
```

#### Rendering the tree to Python

```rust
impl NamespaceNode {
    fn render(&self, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();

        if !self.name.is_empty() {
            // Emit a namespace wrapper class
            writeln!(out, "{}class {}:", pad, self.name).unwrap();
            writeln!(out, "{}    \"\"\"Namespace for {} types.\"\"\"", pad, self.name).unwrap();
            writeln!(out).unwrap();
        }

        let inner_indent = if self.name.is_empty() { indent } else { indent + 1 };
        let inner_pad = "    ".repeat(inner_indent);

        // Emit child namespaces first (they need to be defined before types that
        // reference them, though with `from __future__ import annotations` this
        // is less critical)
        for child in self.children.values() {
            out.push_str(&child.render(inner_indent));
            out.push('\n');
        }

        // Emit types in this namespace
        for type_code in &self.types {
            // Re-indent the rendered type code to sit inside the namespace class
            for line in type_code.lines() {
                if line.trim().is_empty() {
                    writeln!(out).unwrap();
                } else {
                    writeln!(out, "{}{}", inner_pad, line).unwrap();
                }
            }
            out.push('\n');
        }

        out
    }
}
```

**Key insight about indentation**: Each type is currently rendered with zero
indentation (top-level class). When nesting inside namespace classes, we must
re-indent every line. This is a simple prepend of `"    " * depth` to each
non-empty line.

#### `from __future__ import annotations`

The generated file already emits this at the top (line ~8 of generated.py):
```python
from __future__ import annotations
```

This means all annotations are strings, so forward references work without quotes.
A type defined later in the file can be referenced by an earlier type. This is
critical -- it means we don't need topological ordering between namespaces.

#### `model_rebuild()` and namespace classes

Currently `model_rebuild` calls look like:
```python
MyappOrdersV2OrderStatus.model_rebuild()
```

With namespaces, they become:
```python
orders.v2.OrderStatus.model_rebuild()
```

This works in Python because `orders.v2.OrderStatus` is a real class object
accessible via attribute lookup. Pydantic's `model_rebuild()` resolves forward
references in the class's module globals. The key requirement: the namespace classes
must be defined **before** `model_rebuild()` is called (which is already the case --
model rebuilds happen after all type definitions).

**Verified experimentally (Pydantic 2.12.5)**: Because the generated file uses
`from __future__ import annotations`, all annotations are stored as strings and
only evaluated when `model_rebuild()` is called. At that point, Pydantic evaluates
annotations in the module's global namespace, where all namespace classes are already
defined. This means:

1. Forward references work even when a namespace class is referenced before its
   definition in source order.
2. Cross-namespace references like `payments.PaymentInfo` resolve correctly because
   `payments` is a module-global name.
3. `model_rebuild()` works **without** `_types_namespace` in the common case.

For extra safety (e.g., if the generated file is imported piecemeal or types are
used in unusual evaluation contexts), we can optionally pass `_types_namespace`:

```python
_ns = {
    "orders": orders,
    "payments": payments,
    "OrderStatus": OrderStatus,  # top-level types too
    "ReflectapiOption": ReflectapiOption,
    "ReflectapiEmpty": ReflectapiEmpty,
}
orders.v2.OrderStatus.model_rebuild(_types_namespace=_ns)
```

**Recommended approach**: Start with plain `model_rebuild()` (no `_types_namespace`).
If edge cases surface, add the namespace dict as a follow-up. The simpler generation
is preferred since it produces cleaner output and the experimental validation confirms
it works.

### D. Reference Format: Changes to `type_ref_to_python_type`

#### Current behavior

At line ~4275:
```rust
let base_type = improve_class_name(&type_ref.name);
```

This produces `MyappOrdersV2OrderStatus`.

#### New behavior

Replace with a lookup into `TypeNameMapping`:

```rust
// In type_ref_to_python_type, step 3 (user-defined type found in schema):
let base_type = mapping
    .python_ref(&type_ref.name)
    .unwrap_or_else(|| improve_class_name(&type_ref.name));
// improve_class_name kept as fallback for unexpected types
```

This produces `orders.v2.OrderStatus` -- the dotted path.

#### Cross-namespace references

When type A in namespace `orders` references type B in namespace `payments`:

```python
class orders:
    class v2:
        class OrderStatus(BaseModel):
            payment: payments.PaymentInfo  # cross-namespace ref
```

Because `from __future__ import annotations` is active, `payments.PaymentInfo` is a
string at class-definition time and only resolved during `model_rebuild()`. As long as
the `_types_namespace` dict contains `"payments": payments`, this resolves correctly.

#### Same-namespace references

When type A references type B in the same namespace:

```python
class model:
    class Pet(BaseModel):
        kind: model.Kind  # same-namespace, still use full dotted path
```

We always use the full dotted path from root, never relative paths. This is simpler
and avoids ambiguity. The alternative (using just `Kind` for same-namespace
references) would require tracking "current namespace" context through
`type_ref_to_python_type`, which is fragile and unnecessary given
`from __future__ import annotations`.

### E. Client Methods

#### Current behavior

```python
class PetsClient:
    async def list(self, ...) -> ApiResponse[MyapiProtoPaginated[MyapiModelOutputPet]]:
        ...
```

#### New behavior

```python
class PetsClient:
    async def list(self, ...) -> ApiResponse[proto.Paginated[model.output.Pet]]:
        ...
```

The change flows naturally through `render_function` -> `type_ref_to_python_type`,
which now returns dotted paths. No special handling needed for the client class itself.

The client class is defined at top level (not inside a namespace), so it references
types using their full dotted paths from the module root.

### F. Collision Handling

#### Same leaf name, different namespaces (the happy path)

```
myapp::orders::OrderStatus   -> orders.OrderStatus
myapp::payments::OrderStatus -> payments.OrderStatus
```

No collision. This is the entire point of namespacing.

#### Same leaf name, same namespace

This can happen if `consolidate_types` inserts `input`/`output` discriminators:

```
myapp::model::input::Pet -> model.input.Pet
myapp::model::output::Pet -> model.output.Pet
```

These end up in different sub-namespaces (`input` vs `output`), so no collision.

#### True collision: same namespace, same leaf name

This would require two types with identical Rust qualified names after stripping,
which `consolidate_types` prevents. But as a safety net:

```rust
impl TypeNameMapping {
    fn new(...) -> Self {
        // ... build entries ...

        // Detect collisions: same (namespace_path, leaf_name) for different Rust names
        let mut seen: BTreeMap<(Vec<String>, String), String> = BTreeMap::new();
        for (rust_name, (ns_path, leaf)) in &entries {
            let key = (ns_path.clone(), leaf.clone());
            if let Some(existing_rust_name) = seen.get(&key) {
                // Collision! Disambiguate by prepending the distinguishing
                // module segment to the leaf name.
                // This is a fallback -- should rarely trigger.
                // Strategy: use the full PascalCase concatenation for the
                // colliding types (falling back to current behavior).
                warn!("Python namespace collision: {} and {} both map to {}.{}",
                    existing_rust_name, rust_name, ns_path.join("."), leaf);
            }
            seen.insert(key, rust_name.clone());
        }
    }
}
```

For the fallback, we can use the current `improve_class_name` behavior (PascalCase
concatenation) and place the type at the root level, or we can add a distinguishing
segment from the Rust name. This should be rare enough that a simple fallback suffices.

#### Python keyword collisions in namespace names

Namespace segments like `import`, `class`, `type` are Python keywords and can't be
class names. Apply `safe_python_identifier` to each namespace segment:

```rust
let namespace: Vec<String> = parts[..parts.len() - 1]
    .iter()
    .map(|s| safe_python_identifier(s))
    .collect();
```

This might produce `type_` for a module named `type`. Acceptable trade-off.

---

## Changes Required (Function by Function)

### 1. `Config` struct (line ~19)

Add:
```rust
pub namespace_strip_prefix: Option<String>,
```

### 2. New: `TypeNameMapping` struct and `qualified_name_to_python` function

As described above. Add these near `improve_class_name`.

### 3. `generate` function (line ~1095)

After `consolidate_types()`, before the rendering loop:

```rust
let strip_prefix = config.namespace_strip_prefix.clone()
    .unwrap_or_else(|| compute_common_prefix(&all_type_names));
let type_mapping = TypeNameMapping::new(&all_type_names, &implemented_types, &strip_prefix);
```

Thread `type_mapping` through to all render functions and `type_ref_to_python_type`.

Change `rendered_types` key from `improve_class_name(type_def.name())` to
`type_mapping.python_ref(original_type_name)`.

Replace the current "Generate nested class structure" block (line ~1350-1355) with
the new namespace tree build + render.

Change `model_rebuild()` emission to use dotted paths and pass `_types_namespace`.

### 4. `improve_class_name` (line ~3786)

Keep as a fallback for types not in the mapping (shouldn't happen, but defensive).
All call sites that currently use `improve_class_name` switch to `TypeNameMapping`
lookups:

| Call site context | Use `leaf_name()` or `python_ref()`? |
|---|---|
| Class definition name (`class Foo(BaseModel)`) | `leaf_name()` |
| Type annotation in field | `python_ref()` |
| Union member name | `python_ref()` |
| Variant class name derivation | `leaf_name()` of parent + variant suffix |
| Factory class name | `python_ref()` of parent + `Factory` suffix |
| `model_rebuild()` call | `python_ref()` |
| Client return type | `python_ref()` |
| Testing module type list | `python_ref()` |

### 5. `type_ref_to_python_type` (line ~4167)

Add `type_mapping: &TypeNameMapping` parameter. At line ~4275:

```rust
// Old:
let base_type = improve_class_name(&type_ref.name);
// New:
let base_type = type_mapping
    .python_ref(&type_ref.name)
    .map(|s| s.to_string())
    .unwrap_or_else(|| improve_class_name(&type_ref.name));
```

### 6. `render_struct`, `render_enum_without_factory`, and all variant renderers

These all call `improve_class_name(&def.name)` for the class name in the `class`
statement. Change to `type_mapping.leaf_name(&def.name)` since the class definition
lives inside its namespace wrapper.

For variant class names derived from the parent enum, use:
```rust
let variant_class_name = format!(
    "{}{}Variant",
    type_mapping.leaf_name(&enum_def.name).unwrap(),
    to_pascal_case(variant.name()),
);
```

### 7. `generate_nested_class_structure` and related functions (line ~3886)

**Remove entirely**. This ad-hoc namespace grouping (hardcoded `MyapiModel` prefix
detection, manual `Pet` and `Kind` grouping) is replaced by the systematic namespace
tree.

### 8. Templates: `DataClass`, `EnumClass`, etc.

No changes needed to template structs themselves. They already take a `name: String`
field. We just pass the leaf name instead of the concatenated name.

---

## Concrete Example

### Input schema (after consolidate_types)

Type names:
- `myapp::model::Pet` (struct, used as both input and output, so consolidate_types
  will have split it)
- `myapp::model::input::Pet` (input variant)
- `myapp::model::output::Pet` (output variant)
- `myapp::model::Kind` (internally tagged enum)
- `myapp::model::KindDog` (variant struct)
- `myapp::model::KindCat` (variant struct)
- `myapp::proto::PetsListRequest` (struct)
- `myapp::proto::Paginated` (generic struct)
- `myapp::HealthCheckFail` (struct, shallow path)

### Config

```rust
namespace_strip_prefix: None  // auto-detects "myapp"
```

### Computed mapping (strip_prefix = "myapp")

| Rust name | namespace_path | leaf_name | python_ref |
|---|---|---|---|
| `myapp::model::input::Pet` | `["model", "input"]` | `Pet` | `model.input.Pet` |
| `myapp::model::output::Pet` | `["model", "output"]` | `Pet` | `model.output.Pet` |
| `myapp::model::Kind` | `["model"]` | `Kind` | `model.Kind` |
| `myapp::model::KindDog` | `["model"]` | `KindDog` | `model.KindDog` |
| `myapp::model::KindCat` | `["model"]` | `KindCat` | `model.KindCat` |
| `myapp::proto::PetsListRequest` | `["proto"]` | `PetsListRequest` | `proto.PetsListRequest` |
| `myapp::proto::Paginated` | `["proto"]` | `Paginated` | `proto.Paginated` |
| `myapp::HealthCheckFail` | `[]` | `HealthCheckFail` | `HealthCheckFail` |

### Generated output (simplified)

```python
from __future__ import annotations
# ... imports ...


class HealthCheckFail(BaseModel):
    """Generated data model."""
    model_config = ConfigDict(extra="ignore", populate_by_name=True)


class model:
    """Namespace for model types."""

    class input:
        """Namespace for input types."""

        class Pet(BaseModel):
            """Input pet model."""
            model_config = ConfigDict(extra="ignore", populate_by_name=True)
            name: str
            kind: model.Kind
            age: int | None = None

    class output:
        """Namespace for output types."""

        class Pet(BaseModel):
            """Output pet model."""
            model_config = ConfigDict(extra="ignore", populate_by_name=True)
            name: str
            kind: model.Kind
            age: int | None = None

    class KindDog(BaseModel):
        """Dog variant"""
        model_config = ConfigDict(extra="ignore", populate_by_name=True)
        type: Literal["dog"] = "dog"
        breed: str

    class KindCat(BaseModel):
        """Cat variant"""
        model_config = ConfigDict(extra="ignore", populate_by_name=True)
        type: Literal["cat"] = "cat"
        lives: int

    Kind = Union[model.KindDog, model.KindCat]


class proto:
    """Namespace for proto types."""

    class PetsListRequest(BaseModel):
        """Generated data model."""
        model_config = ConfigDict(extra="ignore", populate_by_name=True)
        cursor: str | None = None
        limit: int | None = None

    class Paginated(BaseModel, Generic[T]):
        """Generated data model."""
        model_config = ConfigDict(extra="ignore", populate_by_name=True)
        items: list[T]
        cursor: str | None = None


# Client classes
class AsyncPetsClient:
    async def list(self, request: proto.PetsListRequest, ...) -> ApiResponse[proto.Paginated[model.output.Pet]]:
        ...


# Rebuild models to resolve forward references
try:
    HealthCheckFail.model_rebuild()
    model.input.Pet.model_rebuild()
    model.output.Pet.model_rebuild()
    model.KindDog.model_rebuild()
    model.KindCat.model_rebuild()
    proto.PetsListRequest.model_rebuild()
    proto.Paginated.model_rebuild()
except AttributeError:
    pass
```

---

## Edge Cases

### 1. Types with no module path

`HealthCheckFail` (after stripping `myapp::`) has no `::` separators. It goes to the
root level with no namespace wrapper. This matches current behavior.

### 2. Generic types across namespaces

`proto.Paginated[model.output.Pet]` -- the generic parameter uses the full dotted path.
This works because `type_ref_to_python_type` recursively resolves arguments, and each
argument lookup produces a dotted path.

### 3. Variant classes derived from enums

Variant structs like `KindDog` already exist as separate types in the schema (after
consolidation). They get their own mapping entries and land in the same namespace as
their parent enum. Their names are already unique within the namespace because the
Rust names include the enum prefix.

For *generated* variant classes (those created during `render_enum_without_factory` for
variants with fields), the naming uses `format!("{}{}Variant", leaf_name, variant)`.
These are emitted inline in the same rendered code block as the parent enum, so they
automatically land in the same namespace.

### 4. Factory classes

Factory classes are currently emitted after `model_rebuild()`. Their names should use
the dotted path: `model.KindFactory`. However, factories are generated as standalone
classes, not inside namespace wrappers. Two options:

**Option A** (simpler): Keep factories at top level with dotted-path-derived names:
```python
class model_KindFactory:
    ...
```

**Option B** (consistent): Generate factory classes inside the namespace tree too.
This requires either a second rendering pass or collecting factory code into the
namespace tree before rendering.

**Recommendation**: Option B. Add factory code to the namespace node during the
factory generation phase, then render the full tree (including factories) at the end.
This keeps the API surface clean: `model.KindFactory`.

### 5. `from __future__ import annotations` and namespace self-reference

Inside `class model`, a type annotation `model.Kind` refers to the *enclosing class*
before it's fully defined. With `from __future__ import annotations`, this is fine
because the annotation is never evaluated at class-definition time.

### 6. Empty namespace segments

If `strip_prefix` removes all segments except the leaf, there's no namespace:
```
strip_prefix = "myapp::model"
name = "myapp::model::Pet"
result = ([], "Pet")  # top-level
```

This is correct behavior.

### 7. Deeply nested namespaces

```
myapp::services::orders::v2::internal::OrderStatus
strip_prefix = "myapp"
result = (["services", "orders", "v2", "internal"], "OrderStatus")
```

Produces:
```python
class services:
    class orders:
        class v2:
            class internal:
                class OrderStatus(BaseModel): ...
```

This mirrors the Rust module structure faithfully. Users who want flatter namespaces
can use `rename_types` on the builder to collapse modules before codegen.

### 8. Testing module

The testing module currently lists type names for `MockClient`. Change from:
```python
types = ["MyappOrdersV2OrderStatus", ...]
```
To:
```python
types = ["orders.v2.OrderStatus", ...]
```

The testing utilities need to be able to look up types by dotted path. This may
require the testing module to receive the `_ns` dict or use `eval()` on dotted paths.
Alternatively, build a flat lookup dict:

```python
_all_types = {
    "orders.v2.OrderStatus": orders.v2.OrderStatus,
    "model.Pet": model.Pet,
    ...
}
```

---

## Migration and Backwards Compatibility

This is a **breaking change** to the generated Python API surface. Users referencing
`MyappModelPet` must change to `model.Pet`.

**Mitigation options**:

1. **Feature flag**: Add `pub use_namespaces: bool` to `Config` (default `false`
   initially, flip to `true` in a major version).

2. **Compatibility aliases**: After the namespace tree, emit:
   ```python
   # Backwards compatibility aliases
   MyappModelPet = model.Pet
   MyappModelKind = model.Kind
   ```
   These can be gated behind a `Config` flag and deprecated.

3. **Major version bump**: Since this is a new feature branch, ship it as part of
   a major version release.

**Recommendation**: Feature flag with default `true` for new users, but provide
the compatibility aliases for one major version cycle.

---

## Key Architectural Decision: Render-then-Indent vs. Indent-Aware Rendering

Two approaches for handling indentation inside namespace classes:

**Approach 1: Render-then-Indent (recommended)**

Types are rendered as top-level classes (zero indentation), same as today. The
namespace tree's `render()` method re-indents each line by prepending spaces. This
is how the TypeScript codegen works (namespace blocks wrap already-rendered types).

Pros:
- Minimal changes to existing render functions.
- Template structs unchanged.
- Easy to reason about -- rendering and namespacing are separate concerns.

Cons:
- Re-indentation is a string operation that happens after rendering.
- If any rendered code contains string literals with significant whitespace, the
  re-indentation could corrupt them. In practice, Pydantic models don't have this.

**Approach 2: Indent-Aware Rendering**

Pass an `indent_level: usize` through all render functions and templates, so each
template emits code at the correct indentation depth from the start.

Pros:
- No post-hoc string manipulation.
- Theoretically "cleaner" output.

Cons:
- Massive refactor: every template's `render()` needs an indent parameter.
- Much larger diff for a marginal benefit.

**Decision**: Approach 1. The TypeScript/Rust codegens both use this pattern
successfully. The re-indentation is trivial and well-contained in one function.

---

## Implementation Order

1. Add `qualified_name_to_python` and `compute_common_prefix` (pure functions, easy
   to unit test).
2. Add `TypeNameMapping` struct.
3. Thread `TypeNameMapping` through `type_ref_to_python_type` (biggest refactor --
   touches many function signatures).
4. Build `NamespaceNode` tree and render function.
5. Wire into `generate`: replace `rendered_types` keying, replace
   `generate_nested_class_structure`, update `model_rebuild` emission.
6. Update factory class generation to use namespace tree.
7. Update testing module.
8. Remove `generate_nested_class_structure`, `extract_namespace_from_type_name`,
   `generate_namespace_class` (the old ad-hoc code).
9. Add `Config` fields and feature flag.
10. Update snapshot tests / demo generated.py.
