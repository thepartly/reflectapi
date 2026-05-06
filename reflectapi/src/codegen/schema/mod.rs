// Compiler-only IR for codegen. Some surface here is used only by the
// module's own tests today (e.g. `normalize` strategy variants, accessor
// methods on `SemanticSchema`). They're retained as the supported shape of
// the IR for backends that may consume it next; allow dead_code at the
// module boundary so we don't spuriously warn about unused items in the
// short term.
#![allow(dead_code)]

mod ids;
mod normalize;
mod semantic;
mod symbol;

// Public-to-the-crate surface. Only what backends (currently just Python)
// actually consume is re-exported; everything else stays accessible as
// `self::ids::*`, `self::normalize::*`, etc. for the module's own tests.
pub(crate) use self::ids::{build_schema_ids, SchemaIds};
pub(crate) use self::normalize::{Consolidation, Naming, Normalizer, PipelineBuilder};
pub(crate) use self::semantic::{
    FieldStyle, ResolvedTypeReference, SemanticEnum, SemanticField, SemanticFunction,
    SemanticOutputType, SemanticPrimitive, SemanticSchema, SemanticStruct, SemanticType,
    SemanticTypeParameter, SemanticVariant, SymbolInfo, SymbolTable,
};
pub(crate) use self::symbol::{SymbolId, SymbolKind};
