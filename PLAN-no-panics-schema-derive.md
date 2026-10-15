# Plan: no panics in `reflectapi-schema` and `reflectapi-derive`

Goal: these two crates never panic. A bad input becomes an error the caller
sees: a `None`/`Err` in `reflectapi-schema`, a compile error with a span in
`reflectapi-derive`. Enforced by clippy for these two crates only.

Status: not started. Separate from the dispatch / build-modes change on
`andrey/dispatch-without-axum`.

## Why these two

- `reflectapi-derive` runs inside the compiler. A panic there is an
  "proc macro panicked" crash with no location, instead of a compile error
  pointing at the attribute.
- `reflectapi-schema` runs whenever a schema is built or renamed, including
  in servers that build the schema at startup.

## Sites to fix (non-test code)

### reflectapi-schema

| Where | What | Fix |
|---|---|---|
| `src/rename.rs:106` | `path.file_name().expect("non-empty name")` in `Glob::rename` | `let Some(file_name) = path.file_name() else { return None };`: a name without a final segment is not renamed |
| `src/rename.rs:124` | `file_name.to_str().expect("is utf-8 ...")` | `let Some(name) = file_name.to_str() else { return None };` (it was built from a `&str`, so this never triggers) |
| `src/subst.rs:49` | `assert!(self.arguments.is_empty(), "type parameter cannot have type arguments")` | Keep the substitution but don't panic: substitute and drop the arguments, or leave the reference unchanged. Decide which; document it. A schema with this shape is a derive bug, so a `debug_assert!` plus the fallback is acceptable |
| `src/lib.rs:235` | `assert!(strukt.transparent && strukt.fields.len() == 1)` in `fold_transparent_types::SubstVisitor::new`, then `strukt.fields[0]` | Make `new` return `Option<Self>` and skip structs that don't qualify; the caller already filters transparent single-field structs, so `None` never happens in practice |

Test-only (allowed by the clippy config below, or rewritten to return `Result`):
`src/lib.rs:1515`, `src/rename.rs:159`.

### reflectapi-derive

| Where | What | Fix |
|---|---|---|
| `src/parser.rs:247`, `:249` | `panic!("expected string literal for deprecation note")` in `parse_deprecated_attr`, which already returns `syn::Result` | `return Err(syn::Error::new_spanned(&nv.value, "expected a string literal for the deprecation note"))` |
| `src/parser.rs:241` | `assert!(attr.path().is_ident("deprecated"))` | The caller guarantees it; replace with an early `Err` (or remove) |
| `src/tokenizable_schema.rs:114` | `syn::parse_str::<TokenStream>(transform_callback).unwrap()` | The parser already has a parsed `syn::ExprPath` (`parser.rs:368-381`) and turns it into a `String`; keep it as an `ExprPath` (or `TokenStream`) in the parsed field so no re-parse is needed. Then the field type changes from `String` to `Option<syn::ExprPath>` through the schema-field tokenizer |
| `src/context.rs:60`, `:67`, `:72` | `self.errors.borrow_mut().as_mut().unwrap()` (serde_derive's `Ctxt` pattern: `Option` marks "checked") | Store `errors: RefCell<Vec<syn::Error>>` and a `checked: Cell<bool>`; `check()` sets it and drains the errors. No unwraps |
| `src/context.rs:117` | `panic!("forgot to check for errors")` in `Drop` | `debug_assert!(…)` (a programmer error in the macro itself, caught by the test suite), or drop the check |

## Enforcement, scoped to these two crates

Both crates use `[lints] workspace = true`, and Cargo does not allow adding
crate-specific keys next to it. So deny at the crate root:

```rust
// reflectapi-schema/src/lib.rs and reflectapi-derive/src/lib.rs
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
```

and allow them in tests, workspace-wide (it affects only crates that deny them):

```toml
# clippy.toml (repository root)
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
```

Not covered by these lints: `assert!` and indexing (`v[0]`). The table above
removes the existing ones; `clippy::indexing_slicing` can be denied too if
indexing should be caught in the future (it is noisy).

CI already runs `cargo clippy --all-targets --all-features -- -D warnings`,
so the deny applies on every PR.

## Tests

- `reflectapi-demo/tests/errors/` (trybuild compile-fail tests): add
  - `deprecated_non_string_note.rs`: `#[deprecated = 1]` on a reflected type,
    expecting a compile error at the literal, not a macro panic.
- `reflectapi-schema` unit tests:
  - `Glob::rename` with names that have no final segment returns `None`.
  - `fold_transparent_types` on a schema without qualifying structs is a no-op.
- Run the full suite: snapshot output must not change (no behaviour change for
  valid input).

## Effort

About half a day: roughly ten sites, one small refactor of the derive
`Context`, the transform path type change, and three or four new tests.
