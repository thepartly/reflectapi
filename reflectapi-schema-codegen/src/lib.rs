mod ids;
mod normalize;
mod semantic;
mod symbol;

pub use self::ids::{build_schema_ids, SchemaIds};
pub use self::normalize::{
    CircularDependencyResolutionStage, Consolidation, Naming, NamingResolutionStage,
    NormalizationError, NormalizationPipeline, NormalizationStage, Normalizer, PipelineBuilder,
    ResolutionStrategy, TypeConsolidationStage,
};
pub use self::semantic::*;
pub use self::symbol::{SymbolId, SymbolKind};
