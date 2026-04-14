/// Normalization pipeline for transforming raw schemas into semantic IRs
///
/// This module provides the core normalization passes that transform
/// the raw reflectapi_schema types into validated, immutable semantic
/// representations with deterministic ordering and resolved dependencies.
use crate::symbol::external_symbol_kind;
use crate::{
    FieldStyle, ResolvedTypeReference, SchemaIds, SemanticEnum, SemanticField, SemanticFunction,
    SemanticPrimitive, SemanticSchema, SemanticStruct, SemanticType, SemanticTypeParameter,
    SemanticVariant, SymbolId, SymbolInfo, SymbolKind, SymbolTable,
};
use reflectapi_schema::{
    Enum, Field, Fields, Function, Primitive, Schema, Struct, Type, TypeReference, Typespace,
    Variant,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Trait for individual normalization stages in the pipeline
pub trait NormalizationStage {
    fn name(&self) -> &'static str;
    fn transform(&self, schema: &mut Schema) -> Result<(), Vec<NormalizationError>>;
}

/// Normalization pipeline that applies multiple stages in sequence
#[derive(Default)]
pub struct NormalizationPipeline {
    stages: Vec<Box<dyn NormalizationStage>>,
}

impl NormalizationPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage<S: NormalizationStage + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    pub fn run(&self, schema: &mut Schema) -> Result<(), Vec<NormalizationError>> {
        for stage in &self.stages {
            stage.transform(schema)?;
        }
        Ok(())
    }

    /// Create the standard normalization pipeline.
    ///
    /// Delegates to `PipelineBuilder` with all default settings.
    pub fn standard() -> Self {
        PipelineBuilder::new().build()
    }

    /// Create a codegen-oriented pipeline that only runs CircularDependencyResolution.
    ///
    /// This is designed for use when the caller has already run
    /// `schema.consolidate_types()` and does not want NamingResolution
    /// (which would rename types and create a name-domain mismatch
    /// between the SemanticSchema and the raw Schema used for rendering).
    ///
    /// Delegates to `PipelineBuilder` with consolidation and naming skipped.
    pub fn for_codegen() -> Self {
        PipelineBuilder::new()
            .consolidation(Consolidation::Skip)
            .naming(Naming::Skip)
            .build()
    }
}

// ---------------------------------------------------------------------------
// PipelineBuilder: configurable pipeline construction
// ---------------------------------------------------------------------------

/// Controls whether and how input/output types are merged.
#[derive(Debug, Clone, Default)]
pub enum Consolidation {
    /// Run the standard `TypeConsolidationStage`.
    #[default]
    Standard,
    /// Skip type consolidation entirely.
    Skip,
}

/// Controls how type names are resolved.
#[derive(Default)]
pub enum Naming {
    /// Run the standard `NamingResolutionStage`.
    #[default]
    Standard,
    /// Skip naming resolution entirely.
    Skip,
    /// Use a custom naming stage.
    Custom(Box<dyn NormalizationStage>),
}

impl std::fmt::Debug for Naming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Naming::Standard => write!(f, "Naming::Standard"),
            Naming::Skip => write!(f, "Naming::Skip"),
            Naming::Custom(_) => write!(f, "Naming::Custom(...)"),
        }
    }
}

/// Builder for configuring a normalization pipeline.
///
/// Provides fine-grained control over which normalization stages are included
/// and in what order. The default configuration matches `NormalizationPipeline::standard()`.
///
/// # Examples
///
/// ```rust,ignore
/// // Standard pipeline (equivalent to NormalizationPipeline::standard())
/// let pipeline = PipelineBuilder::new().build();
///
/// // Codegen pipeline (equivalent to NormalizationPipeline::for_codegen())
/// let pipeline = PipelineBuilder::new()
///     .consolidation(Consolidation::Skip)
///     .naming(Naming::Skip)
///     .build();
///
/// // Custom pipeline with extra stages
/// let pipeline = PipelineBuilder::new()
///     .circular_dependency_strategy(ResolutionStrategy::Boxing)
///     .add_stage(MyCustomStage)
///     .build();
/// ```
pub struct PipelineBuilder {
    consolidation: Consolidation,
    naming: Naming,
    circular_dependency_strategy: ResolutionStrategy,
    extra_stages: Vec<Box<dyn NormalizationStage>>,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder {
    /// Create a new builder with default settings (all stages enabled).
    pub fn new() -> Self {
        Self {
            consolidation: Consolidation::default(),
            naming: Naming::default(),
            circular_dependency_strategy: ResolutionStrategy::default(),
            extra_stages: Vec::new(),
        }
    }

    /// Set the consolidation strategy.
    pub fn consolidation(mut self, consolidation: Consolidation) -> Self {
        self.consolidation = consolidation;
        self
    }

    /// Set the naming resolution strategy.
    pub fn naming(mut self, naming: Naming) -> Self {
        self.naming = naming;
        self
    }

    /// Set the circular dependency resolution strategy.
    pub fn circular_dependency_strategy(mut self, strategy: ResolutionStrategy) -> Self {
        self.circular_dependency_strategy = strategy;
        self
    }

    /// Append a custom stage that will run after the built-in stages.
    pub fn add_stage<S: NormalizationStage + 'static>(mut self, stage: S) -> Self {
        self.extra_stages.push(Box::new(stage));
        self
    }

    /// Build the configured `NormalizationPipeline`.
    ///
    /// Stages are added in order:
    /// 1. Type consolidation (if not skipped)
    /// 2. Naming resolution (if not skipped, or custom stage)
    /// 3. Circular dependency resolution (always included)
    /// 4. Any extra stages added via `add_stage()`
    pub fn build(self) -> NormalizationPipeline {
        let mut pipeline = NormalizationPipeline::new();

        match self.consolidation {
            Consolidation::Standard => {
                pipeline = pipeline.add_stage(TypeConsolidationStage);
            }
            Consolidation::Skip => {}
        }

        match self.naming {
            Naming::Standard => {
                pipeline = pipeline.add_stage(NamingResolutionStage);
            }
            Naming::Skip => {}
            Naming::Custom(stage) => {
                pipeline.stages.push(stage);
            }
        }

        pipeline = pipeline.add_stage(CircularDependencyResolutionStage::with_strategy(
            self.circular_dependency_strategy,
        ));

        for stage in self.extra_stages {
            pipeline.stages.push(stage);
        }

        pipeline
    }
}

// ---------------------------------------------------------------------------
// Stage 1: Type Consolidation
// ---------------------------------------------------------------------------

/// Merges input_types and output_types into a single unified types collection.
/// Handles naming conflicts by renaming types with prefixes.
pub struct TypeConsolidationStage;

impl NormalizationStage for TypeConsolidationStage {
    fn name(&self) -> &'static str {
        "TypeConsolidation"
    }

    fn transform(&self, schema: &mut Schema) -> Result<(), Vec<NormalizationError>> {
        let mut consolidated = Typespace::new();
        let mut name_conflicts = HashMap::new();
        // Tracks old_name -> new_name for type reference rewriting
        let mut rename_map: HashMap<String, String> = HashMap::new();

        let mut input_type_names = HashMap::new();
        let mut output_type_names = HashMap::new();

        for ty in schema.input_types.types() {
            let simple_name = extract_simple_name(ty.name());
            input_type_names.insert(simple_name.clone(), ty.clone());

            if output_type_names.contains_key(&simple_name) {
                name_conflicts.insert(simple_name, true);
            }
        }

        for ty in schema.output_types.types() {
            let simple_name = extract_simple_name(ty.name());
            output_type_names.insert(simple_name.clone(), ty.clone());

            if input_type_names.contains_key(&simple_name) {
                name_conflicts.insert(simple_name, true);
            }
        }

        for ty in schema.input_types.types() {
            let simple_name = extract_simple_name(ty.name());
            let mut new_type = ty.clone();

            if name_conflicts.contains_key(&simple_name) {
                let old_name = ty.name().to_string();
                let new_name = format!("input.{}", ty.name().replace("::", "."));
                rename_type(&mut new_type, &new_name);
                rename_map.insert(old_name, new_name);
            }

            consolidated.insert_type(new_type);
        }

        for ty in schema.output_types.types() {
            let simple_name = extract_simple_name(ty.name());
            let mut new_type = ty.clone();

            if name_conflicts.contains_key(&simple_name) {
                let old_name = ty.name().to_string();
                let new_name = format!("output.{}", ty.name().replace("::", "."));
                rename_type(&mut new_type, &new_name);
                rename_map.insert(old_name, new_name);
                consolidated.insert_type(new_type);
            } else if !input_type_names.contains_key(&simple_name) {
                consolidated.insert_type(new_type);
            }
        }

        schema.input_types = consolidated;
        schema.output_types = Typespace::new();

        // Rewrite type references that still point to old names
        if !rename_map.is_empty() {
            for function in &mut schema.functions {
                update_type_reference_in_option(&mut function.input_type, &rename_map);
                update_type_reference_in_option(&mut function.input_headers, &rename_map);
                update_type_reference_in_option(&mut function.output_type, &rename_map);
                update_type_reference_in_option(&mut function.error_type, &rename_map);
            }

            let types_to_update: Vec<_> = schema.input_types.types().cloned().collect();
            schema.input_types = Typespace::new();
            for mut ty in types_to_update {
                update_type_references_in_type(&mut ty, &rename_map);
                schema.input_types.insert_type(ty);
            }
        }

        Ok(())
    }
}

fn extract_simple_name(qualified_name: &str) -> String {
    qualified_name
        .split("::")
        .last()
        .unwrap_or(qualified_name)
        .to_string()
}

fn rename_type(ty: &mut Type, new_name: &str) {
    match ty {
        Type::Struct(s) => s.name = new_name.to_string(),
        Type::Enum(e) => e.name = new_name.to_string(),
        Type::Primitive(p) => p.name = new_name.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Stage 2: Naming Resolution
// ---------------------------------------------------------------------------

/// Sanitizes type names by stripping module paths and handling naming conflicts.
pub struct NamingResolutionStage;

impl NormalizationStage for NamingResolutionStage {
    fn name(&self) -> &'static str {
        "NamingResolution"
    }

    fn transform(&self, schema: &mut Schema) -> Result<(), Vec<NormalizationError>> {
        let mut name_usage: HashMap<String, Vec<String>> = HashMap::new();
        let mut name_conflicts = HashMap::new();

        for ty in schema.input_types.types() {
            let qualified_name = ty.name().to_string();
            let simple_name = extract_simple_name(&qualified_name);

            let entries = name_usage.entry(simple_name.clone()).or_default();
            if !entries.contains(&qualified_name) {
                if !entries.is_empty() {
                    name_conflicts.insert(simple_name.clone(), true);
                }
                entries.push(qualified_name);
            }
        }

        let types_to_update: Vec<_> = schema.input_types.types().cloned().collect();
        schema.input_types = Typespace::new();

        for mut ty in types_to_update {
            let qualified_name = ty.name().to_string();
            let simple_name = extract_simple_name(&qualified_name);

            let resolved_name = if name_conflicts.contains_key(&simple_name) {
                generate_unique_name(&qualified_name)
            } else {
                simple_name
            };

            rename_type(&mut ty, &resolved_name);
            schema.input_types.insert_type(ty);
        }

        update_type_references_in_schema(schema, &name_usage, &name_conflicts);

        Ok(())
    }
}

fn generate_unique_name(qualified_name: &str) -> String {
    let parts: Vec<&str> = qualified_name.split("::").collect();
    if parts.len() < 2 {
        return qualified_name.to_string();
    }

    let type_name = parts.last().unwrap();
    let module_parts: Vec<&str> = parts[..parts.len() - 1].to_vec();

    let non_excluded: Vec<&str> = module_parts
        .iter()
        .filter(|&&part| part != "model" && part != "proto" && !part.is_empty())
        .copied()
        .collect();

    let prefix = if non_excluded.is_empty() {
        module_parts.join("_")
    } else {
        non_excluded
            .iter()
            .map(|s| capitalize_first_letter(s))
            .collect::<Vec<_>>()
            .join("")
    };
    format!("{prefix}{type_name}")
}

fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn update_type_references_in_schema(
    schema: &mut Schema,
    name_usage: &HashMap<String, Vec<String>>,
    name_conflicts: &HashMap<String, bool>,
) {
    let mut name_mapping = HashMap::new();

    for (simple_name, qualified_names) in name_usage {
        if name_conflicts.contains_key(simple_name) {
            for qualified_name in qualified_names {
                let resolved_name = generate_unique_name(qualified_name);
                name_mapping.insert(qualified_name.clone(), resolved_name);
            }
        } else {
            for qualified_name in qualified_names {
                name_mapping.insert(qualified_name.clone(), simple_name.clone());
            }
        }
    }

    for function in &mut schema.functions {
        update_type_reference_in_option(&mut function.input_type, &name_mapping);
        update_type_reference_in_option(&mut function.input_headers, &name_mapping);
        update_type_reference_in_option(&mut function.output_type, &name_mapping);
        update_type_reference_in_option(&mut function.error_type, &name_mapping);
    }

    let types_to_update: Vec<_> = schema.input_types.types().cloned().collect();
    schema.input_types = Typespace::new();

    for mut ty in types_to_update {
        update_type_references_in_type(&mut ty, &name_mapping);
        schema.input_types.insert_type(ty);
    }
}

fn update_type_reference(type_ref: &mut TypeReference, name_mapping: &HashMap<String, String>) {
    if let Some(new_name) = name_mapping.get(&type_ref.name) {
        type_ref.name.clone_from(new_name);
    }

    for arg in &mut type_ref.arguments {
        update_type_reference(arg, name_mapping);
    }
}

fn update_type_reference_in_option(
    type_ref_opt: &mut Option<TypeReference>,
    name_mapping: &HashMap<String, String>,
) {
    if let Some(type_ref) = type_ref_opt {
        update_type_reference(type_ref, name_mapping);
    }
}

fn update_type_references_in_type(ty: &mut Type, name_mapping: &HashMap<String, String>) {
    match ty {
        Type::Struct(s) => match &mut s.fields {
            Fields::Named(fields) | Fields::Unnamed(fields) => {
                for field in fields {
                    update_type_reference(&mut field.type_ref, name_mapping);
                }
            }
            Fields::None => {}
        },
        Type::Enum(e) => {
            for variant in &mut e.variants {
                match &mut variant.fields {
                    Fields::Named(fields) | Fields::Unnamed(fields) => {
                        for field in fields {
                            update_type_reference(&mut field.type_ref, name_mapping);
                        }
                    }
                    Fields::None => {}
                }
            }
        }
        Type::Primitive(p) => {
            if let Some(fallback) = &mut p.fallback {
                update_type_reference(fallback, name_mapping);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stage 3: Circular Dependency Resolution
// ---------------------------------------------------------------------------

/// Detects and resolves circular dependencies using Tarjan's SCC algorithm
/// and configurable resolution strategies.
pub struct CircularDependencyResolutionStage {
    strategy: ResolutionStrategy,
}

#[derive(Debug, Clone, Default)]
pub enum ResolutionStrategy {
    /// Try boxing first, then forward declarations
    #[default]
    Intelligent,
    /// Always use `Box<T>` for self-references
    Boxing,
    /// Always use forward declarations
    ForwardDeclarations,
    /// Make circular references optional
    OptionalBreaking,
    /// Use reference counting for complex cycles
    ReferenceCounted,
}

impl CircularDependencyResolutionStage {
    pub fn new() -> Self {
        Self {
            strategy: ResolutionStrategy::default(),
        }
    }

    pub fn with_strategy(strategy: ResolutionStrategy) -> Self {
        Self { strategy }
    }
}

impl Default for CircularDependencyResolutionStage {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalizationStage for CircularDependencyResolutionStage {
    fn name(&self) -> &'static str {
        "CircularDependencyResolution"
    }

    fn transform(&self, schema: &mut Schema) -> Result<(), Vec<NormalizationError>> {
        let cycles = self.detect_circular_dependencies(schema)?;

        if cycles.is_empty() {
            return Ok(());
        }

        for cycle in cycles {
            self.resolve_cycle(schema, &cycle)?;
        }

        Ok(())
    }
}

impl CircularDependencyResolutionStage {
    fn detect_circular_dependencies(
        &self,
        schema: &Schema,
    ) -> Result<Vec<Vec<String>>, Vec<NormalizationError>> {
        let mut dependencies: HashMap<String, BTreeSet<String>> = HashMap::new();
        let known_types: BTreeSet<String> = schema
            .input_types
            .types()
            .chain(schema.output_types.types())
            .map(|ty| ty.name().to_string())
            .collect();

        for ty in schema
            .input_types
            .types()
            .chain(schema.output_types.types())
        {
            let type_name = ty.name().to_string();
            let mut deps = BTreeSet::new();
            self.collect_type_dependencies(ty, &known_types, &mut deps);
            dependencies.insert(type_name, deps);
        }

        let scc_cycles = self.find_strongly_connected_components(&dependencies);

        let mut cycles = Vec::new();
        for component in scc_cycles {
            if component.len() > 1
                || (component.len() == 1
                    && dependencies
                        .get(&component[0])
                        .is_some_and(|deps| deps.contains(&component[0])))
            {
                cycles.push(component);
            }
        }

        Ok(cycles)
    }

    fn collect_type_dependencies(
        &self,
        ty: &Type,
        known_types: &BTreeSet<String>,
        deps: &mut BTreeSet<String>,
    ) {
        match ty {
            Type::Struct(s) => {
                for field in s.fields() {
                    self.collect_type_ref_dependencies(&field.type_ref, known_types, deps);
                }
            }
            Type::Enum(e) => {
                for variant in e.variants() {
                    for field in variant.fields() {
                        self.collect_type_ref_dependencies(&field.type_ref, known_types, deps);
                    }
                }
            }
            Type::Primitive(p) => {
                if let Some(fallback) = &p.fallback {
                    self.collect_type_ref_dependencies(fallback, known_types, deps);
                }
            }
        }
    }

    fn collect_type_ref_dependencies(
        &self,
        type_ref: &TypeReference,
        known_types: &BTreeSet<String>,
        deps: &mut BTreeSet<String>,
    ) {
        if self.is_dependency_target(&type_ref.name, known_types) {
            deps.insert(type_ref.name.clone());
        }

        for arg in &type_ref.arguments {
            self.collect_type_ref_dependencies(arg, known_types, deps);
        }
    }

    fn is_dependency_target(&self, name: &str, known_types: &BTreeSet<String>) -> bool {
        !self.is_generic_parameter(name) && known_types.contains(name)
    }

    fn is_generic_parameter(&self, name: &str) -> bool {
        name.len() <= 2 && name.chars().all(|c| c.is_ascii_uppercase())
    }

    fn find_strongly_connected_components(
        &self,
        dependencies: &HashMap<String, BTreeSet<String>>,
    ) -> Vec<Vec<String>> {
        let mut index = 0;
        let mut stack = Vec::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut lowlinks: HashMap<String, usize> = HashMap::new();
        let mut on_stack: HashMap<String, bool> = HashMap::new();
        let mut components = Vec::new();

        for node in dependencies.keys() {
            if !indices.contains_key(node) {
                self.strongconnect(
                    node,
                    dependencies,
                    &mut index,
                    &mut stack,
                    &mut indices,
                    &mut lowlinks,
                    &mut on_stack,
                    &mut components,
                );
            }
        }

        components
    }

    #[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
    fn strongconnect(
        &self,
        node: &str,
        dependencies: &HashMap<String, BTreeSet<String>>,
        index: &mut usize,
        stack: &mut Vec<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        on_stack: &mut HashMap<String, bool>,
        components: &mut Vec<Vec<String>>,
    ) {
        indices.insert(node.to_string(), *index);
        lowlinks.insert(node.to_string(), *index);
        *index += 1;
        stack.push(node.to_string());
        on_stack.insert(node.to_string(), true);

        if let Some(deps) = dependencies.get(node) {
            for neighbor in deps {
                if !indices.contains_key(neighbor) {
                    self.strongconnect(
                        neighbor,
                        dependencies,
                        index,
                        stack,
                        indices,
                        lowlinks,
                        on_stack,
                        components,
                    );
                    lowlinks.insert(node.to_string(), lowlinks[node].min(lowlinks[neighbor]));
                } else if *on_stack.get(neighbor).unwrap_or(&false) {
                    lowlinks.insert(node.to_string(), lowlinks[node].min(indices[neighbor]));
                }
            }
        }

        if lowlinks[node] == indices[node] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.insert(w.clone(), false);
                component.push(w.clone());
                if w == node {
                    break;
                }
            }
            if !component.is_empty() {
                components.push(component);
            }
        }
    }

    fn resolve_cycle(
        &self,
        schema: &mut Schema,
        cycle: &[String],
    ) -> Result<(), Vec<NormalizationError>> {
        match self.strategy {
            ResolutionStrategy::Intelligent => {
                if cycle.len() == 1 {
                    self.apply_boxing_strategy(schema, cycle)
                } else {
                    self.apply_forward_declaration_strategy(schema, cycle)
                }
            }
            ResolutionStrategy::Boxing => self.apply_boxing_strategy(schema, cycle),
            ResolutionStrategy::ForwardDeclarations => {
                self.apply_forward_declaration_strategy(schema, cycle)
            }
            ResolutionStrategy::OptionalBreaking => {
                self.apply_optional_breaking_strategy(schema, cycle)
            }
            ResolutionStrategy::ReferenceCounted => {
                self.apply_reference_counting_strategy(schema, cycle)
            }
        }
    }

    /// No-op: Rust schemas already encode `Box<T>` in the type references, so
    /// self-referential types (cycle length 1) and multi-type cycles (A → B → A)
    /// are already representable.  The cycle detection performed by the
    /// `CircularDependencyResolutionStage` is still valuable — downstream codegen
    /// backends (e.g. Python, TypeScript) can query the detected cycles to emit
    /// forward-reference annotations or similar language-specific constructs.
    fn apply_boxing_strategy(
        &self,
        _schema: &mut Schema,
        _cycle: &[String],
    ) -> Result<(), Vec<NormalizationError>> {
        Ok(())
    }

    fn apply_forward_declaration_strategy(
        &self,
        _schema: &mut Schema,
        _cycle: &[String],
    ) -> Result<(), Vec<NormalizationError>> {
        // TODO: Implement forward declarations by creating type aliases
        Ok(())
    }

    fn apply_optional_breaking_strategy(
        &self,
        _schema: &mut Schema,
        _cycle: &[String],
    ) -> Result<(), Vec<NormalizationError>> {
        // TODO: Make certain fields optional to break cycles
        Ok(())
    }

    fn apply_reference_counting_strategy(
        &self,
        _schema: &mut Schema,
        _cycle: &[String],
    ) -> Result<(), Vec<NormalizationError>> {
        // TODO: Wrap cycle references in Rc<RefCell<T>>
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizationError {
    UnresolvedReference {
        name: String,
        referrer: SymbolId,
    },
    CircularDependency {
        cycle: Vec<SymbolId>,
    },
    ConflictingDefinition {
        symbol: SymbolId,
        existing: String,
        new: String,
    },
    InvalidGenericParameter {
        type_name: String,
        parameter: String,
        reason: String,
    },
    ValidationError {
        symbol: SymbolId,
        message: String,
    },
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizationError::UnresolvedReference { name, referrer } => {
                write!(
                    f,
                    "Unresolved type reference '{name}' in symbol {referrer:?}"
                )
            }
            NormalizationError::CircularDependency { cycle } => {
                write!(f, "Circular dependency detected: {cycle:?}")
            }
            NormalizationError::ConflictingDefinition {
                symbol,
                existing,
                new,
            } => {
                write!(
                    f,
                    "Conflicting definition for symbol {symbol:?}: existing '{existing}', new '{new}'"
                )
            }
            NormalizationError::InvalidGenericParameter {
                type_name,
                parameter,
                reason,
            } => {
                write!(
                    f,
                    "Invalid generic parameter '{parameter}' in type '{type_name}': {reason}"
                )
            }
            NormalizationError::ValidationError { symbol, message } => {
                write!(f, "Validation error for symbol {symbol:?}: {message}")
            }
        }
    }
}

impl std::error::Error for NormalizationError {}

// ---------------------------------------------------------------------------
// Normalizer: main pipeline converting Schema -> SemanticSchema
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct NormalizationContext {
    symbol_table: SymbolTable,
    raw_types: HashMap<String, Type>,
    raw_functions: HashMap<String, Function>,
    resolution_cache: HashMap<String, SymbolId>,
    generic_scope: BTreeSet<String>,
    errors: Vec<NormalizationError>,
    /// Compiler-owned ID table, populated in Phase 0
    schema_ids: Option<SchemaIds>,
}

impl Default for NormalizationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalizationContext {
    fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            raw_types: HashMap::new(),
            raw_functions: HashMap::new(),
            resolution_cache: HashMap::new(),
            generic_scope: BTreeSet::new(),
            errors: Vec::new(),
            schema_ids: None,
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn take_errors(&mut self) -> Vec<NormalizationError> {
        std::mem::take(&mut self.errors)
    }
}

/// Main normalizer that converts a raw Schema into a SemanticSchema
pub struct Normalizer {
    context: NormalizationContext,
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            context: NormalizationContext::new(),
        }
    }

    /// Normalize a raw schema into a semantic schema using the standard pipeline.
    pub fn normalize(self, schema: &Schema) -> Result<SemanticSchema, Vec<NormalizationError>> {
        self.normalize_with_pipeline(schema, NormalizationPipeline::standard())
    }

    /// Normalize a raw schema into a semantic schema using a custom pipeline.
    ///
    /// Use `PipelineBuilder` to configure which stages run, or the convenience
    /// methods `NormalizationPipeline::standard()` / `NormalizationPipeline::for_codegen()`.
    pub fn normalize_with_pipeline(
        mut self,
        schema: &Schema,
        pipeline: NormalizationPipeline,
    ) -> Result<SemanticSchema, Vec<NormalizationError>> {
        // Clone so that pipeline stages can mutate without affecting the caller
        let mut schema = schema.clone();

        // Phase 0: Build compiler-owned ID table (side table, not on raw schema)
        let schema_ids = crate::build_schema_ids(&schema);
        self.context.schema_ids = Some(schema_ids);

        // Capture original type names BEFORE the pipeline transforms them.
        // NamingResolution (if present in the pipeline) will strip module
        // paths, so we need to map short names back to qualified names.
        let pre_norm_names: Vec<String> = schema
            .input_types
            .types()
            .chain(schema.output_types.types())
            .map(|t| t.name().to_string())
            .collect();

        // Run the caller-provided pipeline
        pipeline.run(&mut schema)?;

        // Build the original_names reverse mapping.
        // When NamingResolution runs, it strips module paths (e.g.
        // "my_module::MyType" -> "MyType"). We map the short name back
        // to the pre-pipeline qualified name.
        // When NamingResolution is NOT in the pipeline, names are unchanged
        // and the mapping is identity — the unwrap_or fallback handles this.
        let mut original_names: HashMap<String, String> = HashMap::new();
        for pre_name in &pre_norm_names {
            let short = pre_name.split("::").last().unwrap_or(pre_name);
            original_names
                .entry(short.to_string())
                .or_insert_with(|| pre_name.clone());
        }

        // Phase 1: Symbol Discovery
        self.discover_symbols(&schema)?;

        // Phase 2: Type Resolution
        self.resolve_types()?;

        // Phase 3: Dependency Analysis
        self.analyze_dependencies()?;

        // Phase 4: Semantic Validation
        self.validate_semantics()?;

        // Phase 5: IR Construction
        self.build_semantic_ir(&schema, &original_names)
    }

    fn discover_symbols(&mut self, schema: &Schema) -> Result<(), Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set before discovery");

        let schema_info = SymbolInfo {
            id: ids.schema_id.clone(),
            name: schema.name.clone(),
            path: ids.schema_id.path.clone(),
            kind: SymbolKind::Schema,
            resolved: false,
            dependencies: BTreeSet::new(),
        };
        self.context.symbol_table.register(schema_info);

        for function in &schema.functions {
            let func_id = ids
                .functions
                .get(&function.name)
                .cloned()
                .unwrap_or_else(|| {
                    SymbolId::new(SymbolKind::Endpoint, vec![function.name.clone()])
                });
            let function_info = SymbolInfo {
                id: func_id.clone(),
                name: function.name.clone(),
                path: func_id.path.clone(),
                kind: SymbolKind::Endpoint,
                resolved: false,
                dependencies: BTreeSet::new(),
            };
            self.context.symbol_table.register(function_info);
            self.context
                .raw_functions
                .insert(function.name.clone(), function.clone());
        }

        self.discover_types_from_typespace(&schema.input_types);
        self.discover_types_from_typespace(&schema.output_types);

        if self.context.has_errors() {
            return Err(self.context.take_errors());
        }

        Ok(())
    }

    fn discover_types_from_typespace(&mut self, typespace: &Typespace) {
        for ty in typespace.types() {
            self.discover_type_symbols(ty);
        }
    }

    fn discover_type_symbols(&mut self, ty: &Type) {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let name = ty.name().to_string();
        let kind = match ty {
            Type::Primitive(_) => SymbolKind::Primitive,
            Type::Struct(_) => SymbolKind::Struct,
            Type::Enum(_) => SymbolKind::Enum,
        };
        let id = ids.type_id(&name);

        let symbol_info = SymbolInfo {
            id: id.clone(),
            name: name.clone(),
            path: id.path.clone(),
            kind,
            resolved: false,
            dependencies: BTreeSet::new(),
        };

        self.context.symbol_table.register(symbol_info);
        self.context.raw_types.insert(name.clone(), ty.clone());

        match ty {
            Type::Struct(s) => self.discover_struct_symbols(&name, s),
            Type::Enum(e) => self.discover_enum_symbols(&name, e),
            Type::Primitive(_) => {}
        }
    }

    fn discover_struct_symbols(&mut self, parent_fqn: &str, strukt: &Struct) {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        for field in strukt.fields() {
            let field_id = ids.member_id(parent_fqn, &field.name);
            let field_info = SymbolInfo {
                id: field_id.clone(),
                name: field.name.clone(),
                path: field_id.path.clone(),
                kind: SymbolKind::Field,
                resolved: false,
                dependencies: BTreeSet::new(),
            };
            self.context.symbol_table.register(field_info);
        }
    }

    fn discover_enum_symbols(&mut self, parent_fqn: &str, enm: &Enum) {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        for variant in enm.variants() {
            let variant_id = ids.member_id(parent_fqn, &variant.name);
            let variant_info = SymbolInfo {
                id: variant_id.clone(),
                name: variant.name.clone(),
                path: variant_id.path.clone(),
                kind: SymbolKind::Variant,
                resolved: false,
                dependencies: BTreeSet::new(),
            };
            self.context.symbol_table.register(variant_info);

            let variant_fqn = variant_id.qualified_name();
            for field in variant.fields() {
                let field_id = ids.member_id(&variant_fqn, &field.name);
                let field_info = SymbolInfo {
                    id: field_id.clone(),
                    name: field.name.clone(),
                    path: field_id.path.clone(),
                    kind: SymbolKind::Field,
                    resolved: false,
                    dependencies: BTreeSet::new(),
                };
                self.context.symbol_table.register(field_info);
            }
        }
    }

    fn resolve_types(&mut self) -> Result<(), Vec<NormalizationError>> {
        for symbol_info in self.context.symbol_table.symbols.values() {
            if !matches!(
                symbol_info.kind,
                SymbolKind::Struct
                    | SymbolKind::Enum
                    | SymbolKind::Primitive
                    | SymbolKind::TypeAlias
            ) {
                continue;
            }
            self.context
                .resolution_cache
                .insert(symbol_info.name.clone(), symbol_info.id.clone());

            let qualified_name = symbol_info.id.qualified_name();
            if qualified_name != symbol_info.name {
                self.context
                    .resolution_cache
                    .insert(qualified_name, symbol_info.id.clone());
            }
        }

        // Clone the ID lookups upfront to avoid borrow conflict with &mut self
        let ids = self
            .context
            .schema_ids
            .clone()
            .expect("schema_ids must be set");
        for (func_name, function) in &self.context.raw_functions.clone() {
            let func_id =
                ids.functions.get(func_name).cloned().unwrap_or_else(|| {
                    SymbolId::new(SymbolKind::Endpoint, vec![func_name.clone()])
                });
            self.resolve_function_references(&func_id, function);
        }

        for (type_name, ty) in &self.context.raw_types.clone() {
            let type_id = ids.type_id(type_name);
            self.resolve_type_references(&type_id, ty);
        }

        if self.context.has_errors() {
            return Err(self.context.take_errors());
        }

        Ok(())
    }

    fn resolve_function_references(&mut self, function_id: &SymbolId, function: &Function) {
        if let Some(input_type) = &function.input_type {
            self.resolve_single_reference(function_id, input_type);
        }
        if let Some(input_headers) = &function.input_headers {
            self.resolve_single_reference(function_id, input_headers);
        }
        if let Some(output_type) = &function.output_type {
            self.resolve_single_reference(function_id, output_type);
        }
        if let Some(error_type) = &function.error_type {
            self.resolve_single_reference(function_id, error_type);
        }
    }

    fn resolve_type_references(&mut self, type_id: &SymbolId, ty: &Type) {
        let generic_params: BTreeSet<String> = ty.parameters().map(|p| p.name.clone()).collect();
        self.context.generic_scope.extend(generic_params.clone());

        match ty {
            Type::Struct(s) => {
                for field in s.fields() {
                    self.resolve_field_references(type_id, field);
                }
            }
            Type::Enum(e) => {
                for variant in e.variants() {
                    for field in variant.fields() {
                        self.resolve_field_references(type_id, field);
                    }
                }
            }
            Type::Primitive(p) => {
                if let Some(fallback) = &p.fallback {
                    self.resolve_single_reference(type_id, fallback);
                }
            }
        }

        for param in generic_params {
            self.context.generic_scope.remove(&param);
        }
    }

    fn resolve_field_references(&mut self, owner_id: &SymbolId, field: &Field) {
        self.resolve_single_reference(owner_id, &field.type_ref);
    }

    fn resolve_single_reference(&mut self, referrer: &SymbolId, type_ref: &TypeReference) {
        if self.context.generic_scope.contains(&type_ref.name) {
            for arg in &type_ref.arguments {
                self.resolve_single_reference(referrer, arg);
            }
            return;
        }

        if let Some(target_id) = self.resolve_global_type_reference(&type_ref.name) {
            self.context
                .symbol_table
                .add_dependency(referrer.clone(), target_id);
        }
        // Unresolved references are silently ignored for now -
        // they'll be handled as placeholders in IR building

        for arg in &type_ref.arguments {
            self.resolve_single_reference(referrer, arg);
        }
    }

    fn resolve_global_type_reference(&self, name: &str) -> Option<SymbolId> {
        self.context
            .resolution_cache
            .get(name)
            .cloned()
            .or_else(|| {
                external_symbol_kind(name)
                    .map(|kind| SymbolId::new(kind, name.split("::").map(str::to_string).collect()))
            })
    }

    fn analyze_dependencies(&mut self) -> Result<(), Vec<NormalizationError>> {
        match self.context.symbol_table.topological_sort() {
            Ok(_) => Ok(()),
            Err(_cycle) => {
                // Cycles may be expected after CircularDependencyResolutionStage
                Ok(())
            }
        }
    }

    fn validate_semantics(&mut self) -> Result<(), Vec<NormalizationError>> {
        // TODO: Add semantic validation passes
        if self.context.has_errors() {
            return Err(self.context.take_errors());
        }
        Ok(())
    }

    fn build_semantic_ir(
        self,
        schema: &Schema,
        original_names: &HashMap<String, String>,
    ) -> Result<SemanticSchema, Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let mut semantic_types = BTreeMap::new();
        let mut semantic_functions = BTreeMap::new();

        let sorted_symbols = match self.context.symbol_table.topological_sort() {
            Ok(sorted) => sorted,
            Err(_cycle) => self.context.symbol_table.symbols.keys().cloned().collect(),
        };

        for symbol_id in sorted_symbols {
            // Look up the type by the symbol's name (raw_types keyed by name)
            if let Some(symbol_info) = self.context.symbol_table.symbols.get(&symbol_id) {
                if let Some(raw_type) = self.context.raw_types.get(&symbol_info.name) {
                    let semantic_type = self.build_semantic_type(raw_type, original_names)?;
                    semantic_types.insert(symbol_id, semantic_type);
                }
            }
        }

        for (func_name, raw_function) in &self.context.raw_functions {
            let func_id =
                ids.functions.get(func_name).cloned().unwrap_or_else(|| {
                    SymbolId::new(SymbolKind::Endpoint, vec![func_name.clone()])
                });
            let semantic_function = self.build_semantic_function(raw_function)?;
            semantic_functions.insert(func_id, semantic_function);
        }

        Ok(SemanticSchema {
            id: ids.schema_id.clone(),
            name: schema.name.clone(),
            description: schema.description.clone(),
            functions: semantic_functions,
            types: semantic_types,
            symbol_table: self.context.symbol_table,
        })
    }

    fn build_semantic_type(
        &self,
        raw_type: &Type,
        original_names: &HashMap<String, String>,
    ) -> Result<SemanticType, Vec<NormalizationError>> {
        match raw_type {
            Type::Primitive(p) => Ok(SemanticType::Primitive(
                self.build_semantic_primitive(p, original_names)?,
            )),
            Type::Struct(s) => Ok(SemanticType::Struct(
                self.build_semantic_struct(s, original_names)?,
            )),
            Type::Enum(e) => Ok(SemanticType::Enum(
                self.build_semantic_enum(e, original_names)?,
            )),
        }
    }

    fn build_semantic_primitive(
        &self,
        primitive: &Primitive,
        original_names: &HashMap<String, String>,
    ) -> Result<SemanticPrimitive, Vec<NormalizationError>> {
        let fallback = primitive
            .fallback
            .as_ref()
            .and_then(|tr| self.resolve_global_type_reference(&tr.name));

        let original_name = original_names
            .get(&primitive.name)
            .cloned()
            .unwrap_or_else(|| primitive.name.clone());

        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        Ok(SemanticPrimitive {
            id: ids.type_id(&primitive.name),
            name: primitive.name.clone(),
            original_name,
            description: primitive.description.clone(),
            parameters: primitive
                .parameters
                .iter()
                .map(|p| SemanticTypeParameter {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    bounds: vec![],
                    default: None,
                })
                .collect(),
            fallback,
            codegen_config: primitive.codegen_config.clone(),
        })
    }

    fn build_semantic_struct(
        &self,
        strukt: &Struct,
        original_names: &HashMap<String, String>,
    ) -> Result<SemanticStruct, Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let mut fields = BTreeMap::new();

        for field in strukt.fields() {
            let field_id = ids.member_id(&strukt.name, &field.name);
            let semantic_field = self.build_semantic_field(&strukt.name, field)?;
            fields.insert(field_id, semantic_field);
        }

        let original_name = original_names
            .get(&strukt.name)
            .cloned()
            .unwrap_or_else(|| strukt.name.clone());

        Ok(SemanticStruct {
            id: ids.type_id(&strukt.name),
            name: strukt.name.clone(),
            original_name,
            serde_name: strukt.serde_name.clone(),
            description: strukt.description.clone(),
            parameters: strukt
                .parameters
                .iter()
                .map(|p| SemanticTypeParameter {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    bounds: vec![],
                    default: None,
                })
                .collect(),
            fields,
            transparent: strukt.transparent,
            is_tuple: strukt.is_tuple(),
            is_unit: strukt.is_unit(),
            codegen_config: strukt.codegen_config.clone(),
        })
    }

    fn build_semantic_enum(
        &self,
        enm: &Enum,
        original_names: &HashMap<String, String>,
    ) -> Result<SemanticEnum, Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let mut variants = BTreeMap::new();

        for variant in enm.variants() {
            let variant_id = ids.member_id(&enm.name, &variant.name);
            let semantic_variant = self.build_semantic_variant(&enm.name, variant)?;
            variants.insert(variant_id, semantic_variant);
        }

        let original_name = original_names
            .get(&enm.name)
            .cloned()
            .unwrap_or_else(|| enm.name.clone());

        Ok(SemanticEnum {
            id: ids.type_id(&enm.name),
            name: enm.name.clone(),
            original_name,
            serde_name: enm.serde_name.clone(),
            description: enm.description.clone(),
            parameters: enm
                .parameters
                .iter()
                .map(|p| SemanticTypeParameter {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    bounds: vec![],
                    default: None,
                })
                .collect(),
            variants,
            representation: enm.representation.clone(),
            codegen_config: enm.codegen_config.clone(),
        })
    }

    fn build_semantic_field(
        &self,
        parent_fqn: &str,
        field: &Field,
    ) -> Result<SemanticField, Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let resolved_type_ref = self.build_resolved_type_reference(&field.type_ref)?;

        Ok(SemanticField {
            id: ids.member_id(parent_fqn, &field.name),
            name: field.name.clone(),
            serde_name: field.serde_name.clone(),
            description: field.description.clone(),
            deprecation_note: field.deprecation_note.clone(),
            type_ref: resolved_type_ref,
            required: field.required,
            flattened: field.flattened,
            transform_callback: field.transform_callback.clone(),
        })
    }

    fn build_semantic_variant(
        &self,
        enum_fqn: &str,
        variant: &Variant,
    ) -> Result<SemanticVariant, Vec<NormalizationError>> {
        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        let variant_id = ids.member_id(enum_fqn, &variant.name);
        let variant_fqn = variant_id.qualified_name();
        let mut fields = BTreeMap::new();

        for field in variant.fields() {
            let field_id = ids.member_id(&variant_fqn, &field.name);
            let semantic_field = self.build_semantic_field(&variant_fqn, field)?;
            fields.insert(field_id, semantic_field);
        }

        let field_style = match &variant.fields {
            Fields::Named(_) => FieldStyle::Named,
            Fields::Unnamed(_) => FieldStyle::Unnamed,
            Fields::None => FieldStyle::Unit,
        };

        Ok(SemanticVariant {
            id: variant_id,
            name: variant.name.clone(),
            serde_name: variant.serde_name.clone(),
            description: variant.description.clone(),
            fields,
            discriminant: variant.discriminant,
            untagged: variant.untagged,
            field_style,
        })
    }

    fn build_semantic_function(
        &self,
        function: &Function,
    ) -> Result<SemanticFunction, Vec<NormalizationError>> {
        let input_type = function
            .input_type
            .as_ref()
            .and_then(|tr| self.resolve_global_type_reference(&tr.name));
        let input_headers = function
            .input_headers
            .as_ref()
            .and_then(|tr| self.resolve_global_type_reference(&tr.name));
        let output_type = function
            .output_type
            .as_ref()
            .and_then(|tr| self.resolve_global_type_reference(&tr.name));
        let error_type = function
            .error_type
            .as_ref()
            .and_then(|tr| self.resolve_global_type_reference(&tr.name));

        let ids = self
            .context
            .schema_ids
            .as_ref()
            .expect("schema_ids must be set");
        Ok(SemanticFunction {
            id: ids
                .functions
                .get(&function.name)
                .cloned()
                .unwrap_or_else(|| {
                    SymbolId::new(SymbolKind::Endpoint, vec![function.name.clone()])
                }),
            name: function.name.clone(),
            path: function.path.clone(),
            description: function.description.clone(),
            deprecation_note: function.deprecation_note.clone(),
            input_type,
            input_headers,
            output_type,
            error_type,
            serialization: function.serialization.clone(),
            readonly: function.readonly,
            tags: function.tags.clone(),
        })
    }

    fn build_resolved_type_reference(
        &self,
        type_ref: &TypeReference,
    ) -> Result<ResolvedTypeReference, Vec<NormalizationError>> {
        let is_likely_generic = !type_ref.name.contains("::");

        let target = if let Some(target) = self.resolve_global_type_reference(&type_ref.name) {
            target
        } else if is_likely_generic {
            SymbolId::new(SymbolKind::TypeAlias, vec![type_ref.name.clone()])
        } else {
            SymbolId::new(SymbolKind::Struct, vec![type_ref.name.replace("::", "_")])
        };

        let mut resolved_args = Vec::new();
        for arg in &type_ref.arguments {
            resolved_args.push(self.build_resolved_type_reference(arg)?);
        }

        Ok(ResolvedTypeReference::new(
            target,
            resolved_args,
            type_ref.name.clone(),
        ))
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use reflectapi_schema::{
        Enum, Field, Fields, Function, Representation, Schema, Struct, Type, TypeReference,
        Typespace, Variant,
    };

    #[test]
    fn test_basic_normalization() {
        let mut schema = Schema::new();
        schema.name = "TestSchema".to_string();

        let user_struct = Struct::new("User");
        let user_type = Type::Struct(user_struct);

        let mut input_types = Typespace::new();
        input_types.insert_type(user_type);
        schema.input_types = input_types;

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);

        assert!(
            result.is_ok(),
            "Normalization should succeed for simple schema"
        );

        let semantic_schema = result.unwrap();
        assert_eq!(semantic_schema.name, "TestSchema");
        assert_eq!(semantic_schema.types.len(), 1);
    }

    #[test]
    fn test_unresolved_reference_handled_gracefully() {
        let mut schema = Schema::new();
        schema.name = "TestSchema".to_string();

        let mut function = Function::new("test_function".to_string());
        function.input_type = Some(TypeReference::new("NonExistentType", vec![]));
        schema.functions.push(function);

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);

        assert!(
            result.is_ok(),
            "Normalization should handle unresolved references gracefully"
        );

        let semantic_schema = result.unwrap();
        assert!(!semantic_schema.functions.is_empty());
    }

    #[test]
    fn test_normalize_with_functions_and_types() {
        let mut schema = Schema::new();
        schema.name = "API".to_string();

        // Add types
        let mut user_struct = Struct::new("api::User");
        user_struct.fields = Fields::Named(vec![
            Field::new("name".into(), "std::string::String".into()),
            Field::new("age".into(), "u32".into()),
        ]);
        schema.input_types.insert_type(user_struct.into());

        let mut error_enum = Enum::new("api::Error".into());
        error_enum.representation = Representation::Internal { tag: "type".into() };
        error_enum.variants = vec![
            Variant::new("NotFound".into()),
            Variant::new("Forbidden".into()),
        ];
        schema.output_types.insert_type(error_enum.into());

        // Add a function referencing both types
        let mut function = Function::new("get_user".into());
        function.input_type = Some(TypeReference::new("api::User", vec![]));
        function.error_type = Some(TypeReference::new("api::Error", vec![]));
        schema.functions.push(function);

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);
        assert!(result.is_ok(), "Normalization failed: {:?}", result.err());

        let semantic = result.unwrap();
        assert_eq!(semantic.types.len(), 2);
        assert_eq!(semantic.functions.len(), 1);

        // Verify the function has resolved type references
        let func = semantic.functions.values().next().unwrap();
        assert!(func.input_type.is_some());
        assert!(func.error_type.is_some());
    }

    #[test]
    fn test_normalize_function_with_input_headers() {
        let mut schema = Schema::new();
        schema.name = "API".to_string();

        let headers_struct = Struct::new("Headers");
        schema.input_types.insert_type(headers_struct.into());

        let body_struct = Struct::new("Body");
        schema.input_types.insert_type(body_struct.into());

        let mut function = Function::new("do_thing".into());
        function.input_type = Some(TypeReference::new("Body", vec![]));
        function.input_headers = Some(TypeReference::new("Headers", vec![]));
        schema.functions.push(function);

        let normalizer = Normalizer::new();
        let semantic = normalizer.normalize(&schema).unwrap();

        let func = semantic.functions.values().next().unwrap();
        assert!(func.input_type.is_some());
        assert!(func.input_headers.is_some());
    }

    #[test]
    fn test_type_consolidation_shared_name() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Same simple name in both typespaces triggers conflict renaming
        let input_struct = Struct::new("Shared");
        let output_struct = Struct::new("Shared");
        schema.input_types.insert_type(input_struct.into());
        schema.output_types.insert_type(output_struct.into());

        let stage = TypeConsolidationStage;
        stage.transform(&mut schema).unwrap();

        // Both get prefixed since they share a simple name
        let type_names: Vec<_> = schema
            .input_types
            .types()
            .map(|t| t.name().to_string())
            .collect();
        assert!(
            type_names.contains(&"input.Shared".to_string()),
            "Expected input.Shared, got: {type_names:?}"
        );
        assert!(
            type_names.contains(&"output.Shared".to_string()),
            "Expected output.Shared, got: {type_names:?}"
        );
        assert!(schema.output_types.is_empty());
    }

    #[test]
    fn test_type_consolidation_conflict_renaming() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Different types sharing simple name get renamed
        let mut input_struct = Struct::new("Foo");
        input_struct.description = "input version".into();
        let mut output_struct = Struct::new("Foo");
        output_struct.description = "output version".into();
        // Make them different so they're not deduplicated
        output_struct.fields = Fields::Named(vec![Field::new("x".into(), "u32".into())]);

        schema.input_types.insert_type(input_struct.into());
        schema.output_types.insert_type(output_struct.into());

        let stage = TypeConsolidationStage;
        stage.transform(&mut schema).unwrap();

        let type_names: Vec<_> = schema
            .input_types
            .types()
            .map(|t| t.name().to_string())
            .collect();
        assert!(
            type_names.contains(&"input.Foo".to_string())
                || type_names.contains(&"output.Foo".to_string()),
            "Expected conflict renaming, got: {type_names:?}"
        );
    }

    #[test]
    fn test_build_schema_ids_idempotent() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut user_struct = Struct::new("User");
        user_struct.fields = Fields::Named(vec![Field::new("id".into(), "u64".into())]);
        schema.input_types.insert_type(user_struct.into());

        // Run twice — should produce identical results
        let ids_first = crate::build_schema_ids(&schema);
        let ids_second = crate::build_schema_ids(&schema);

        let user_id_first = ids_first.type_id("User");
        let user_id_second = ids_second.type_id("User");
        assert_eq!(
            user_id_first, user_id_second,
            "build_schema_ids should be idempotent"
        );
    }

    #[test]
    fn test_build_schema_ids_enum_variants_and_fields() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut enm = Enum::new("Status".into());
        let mut variant = Variant::new("Active".into());
        variant.fields = Fields::Named(vec![Field::new(
            "since".into(),
            "std::string::String".into(),
        )]);
        enm.variants = vec![variant, Variant::new("Inactive".into())];
        schema.input_types.insert_type(enm.into());

        let ids = crate::build_schema_ids(&schema);

        let enum_id = ids.type_id("Status");
        assert!(!enum_id.is_unknown(), "Enum should have a non-unknown id");

        let active_id = ids.member_id("Status", "Active");
        assert!(
            !active_id.is_unknown(),
            "Variant 'Active' should have a non-unknown id"
        );

        let inactive_id = ids.member_id("Status", "Inactive");
        assert!(
            !inactive_id.is_unknown(),
            "Variant 'Inactive' should have a non-unknown id"
        );

        // Check variant field ID
        let active_fqn = active_id.qualified_name();
        let since_id = ids.member_id(&active_fqn, "since");
        assert!(
            !since_id.is_unknown(),
            "Field 'since' should have a non-unknown id"
        );

        // Check paths are structured correctly
        assert_eq!(active_id.path.last().unwrap(), "Active");
        assert!(
            since_id.path.contains(&"Active".to_string()),
            "Field path should include parent variant: {:?}",
            since_id.path
        );
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Node { children: Vec<Node> } - self-referential
        let mut node_struct = Struct::new("Node");
        node_struct.fields = Fields::Named(vec![Field::new(
            "children".into(),
            TypeReference::new("std::vec::Vec", vec![TypeReference::new("Node", vec![])]),
        )]);
        schema.input_types.insert_type(node_struct.into());

        let stage = CircularDependencyResolutionStage::new();
        // Should detect the cycle but not fail (strategies are stubs)
        let result = stage.transform(&mut schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_schema_normalization() {
        let schema = Schema::new();
        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);
        assert!(result.is_ok());

        let semantic = result.unwrap();
        assert!(semantic.types.is_empty());
        assert!(semantic.functions.is_empty());
    }

    #[test]
    fn test_naming_resolution_all_conflicting_types_have_references_rewritten() {
        // Regression: NamingResolutionStage only tracked the first qualified name
        // per simple name in name_usage, leaving references to the second conflicting
        // type dangling after rename.
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Two types sharing simple name "Foo" in different modules
        let a_foo = Struct::new("a::Foo");
        let b_foo = Struct::new("b::Foo");
        schema.input_types.insert_type(a_foo.into());
        schema.input_types.insert_type(b_foo.into());

        // Function referencing BOTH types
        let mut func1 = Function::new("use_a_foo".into());
        func1.input_type = Some(TypeReference::new("a::Foo", vec![]));
        schema.functions.push(func1);

        let mut func2 = Function::new("use_b_foo".into());
        func2.input_type = Some(TypeReference::new("b::Foo", vec![]));
        schema.functions.push(func2);

        let stage = NamingResolutionStage;
        stage.transform(&mut schema).unwrap();

        // Collect all type names defined in the schema
        let type_names: std::collections::HashSet<String> = schema
            .input_types
            .types()
            .map(|t| t.name().to_string())
            .collect();

        // Both function references must point to names that exist in the schema
        for func in &schema.functions {
            if let Some(ref input_type) = func.input_type {
                assert!(
                    type_names.contains(&input_type.name),
                    "Function '{}' references type '{}' which doesn't exist in schema. Available: {:?}",
                    func.name, input_type.name, type_names
                );
            }
        }
    }

    #[test]
    fn test_generate_unique_name_excluded_modules_no_collision() {
        // Regression: when all module parts are in the exclusion list ("model", "proto"),
        // the fallback was module_parts[0], causing "model::Foo" and "model::proto::Foo"
        // to both become "ModelFoo". Now uses joined fallback to avoid collisions.
        let name1 = generate_unique_name("model::Foo");
        let name2 = generate_unique_name("model::proto::Foo");

        assert_ne!(
            name1, name2,
            "model::Foo and model::proto::Foo must produce different names, got '{name1}' and '{name2}'"
        );
    }

    #[test]
    fn test_generate_unique_name_with_non_excluded_module() {
        // Normal case: module part not in exclusion list is used as prefix
        let name = generate_unique_name("billing::Invoice");
        assert_eq!(name, "BillingInvoice");
    }

    #[test]
    fn test_self_referential_type_normalizes_successfully() {
        // A self-referential type (cycle of length 1) should pass through the
        // full Normalizer pipeline without error.  In Rust the schema already
        // records Box<T> wrappers, so the boxing strategy is intentionally a
        // no-op — the cycle is detected but does not block normalization.
        let mut schema = Schema::new();
        schema.name = "TreeSchema".to_string();

        // TreeNode has a field `children` of type Vec<TreeNode> (indirect
        // self-reference via a container — already broken by Vec) and a field
        // `parent` that directly references TreeNode (direct self-reference,
        // which in real Rust code would be Box<TreeNode>).
        let mut tree_node = Struct::new("TreeNode");
        tree_node.fields = Fields::Named(vec![
            Field::new("label".into(), "std::string::String".into()),
            Field::new(
                "children".into(),
                TypeReference::new(
                    "std::vec::Vec",
                    vec![TypeReference::new("TreeNode", vec![])],
                ),
            ),
            Field::new(
                "parent".into(),
                TypeReference::new(
                    "std::boxed::Box",
                    vec![TypeReference::new("TreeNode", vec![])],
                ),
            ),
        ]);
        schema.input_types.insert_type(tree_node.into());

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);

        assert!(
            result.is_ok(),
            "Self-referential type should not prevent normalization: {:?}",
            result.err()
        );

        let semantic = result.unwrap();
        assert_eq!(semantic.types.len(), 1, "TreeNode type should be present");

        // Verify the type round-tripped with the expected name
        let tree_node_type = semantic.types.values().next().unwrap();
        match tree_node_type {
            SemanticType::Struct(s) => {
                assert_eq!(s.name, "TreeNode");
                assert_eq!(s.fields.len(), 3, "All three fields should survive");
            }
            other => panic!("Expected Struct, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn test_multi_type_cycle_normalizes_successfully() {
        // A → B → A cycle (length 2) should also pass through normalization
        // without error.  The forward-declaration strategy is likewise a no-op
        // for Rust schemas.
        let mut schema = Schema::new();
        schema.name = "CycleSchema".to_string();

        // Department references Employee, Employee references Department
        let mut department = Struct::new("Department");
        department.fields = Fields::Named(vec![
            Field::new("name".into(), "std::string::String".into()),
            Field::new("manager".into(), TypeReference::new("Employee", vec![])),
        ]);

        let mut employee = Struct::new("Employee");
        employee.fields = Fields::Named(vec![
            Field::new("name".into(), "std::string::String".into()),
            Field::new(
                "department".into(),
                TypeReference::new("Department", vec![]),
            ),
        ]);

        schema.input_types.insert_type(department.into());
        schema.input_types.insert_type(employee.into());

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);

        assert!(
            result.is_ok(),
            "Multi-type cycle should not prevent normalization: {:?}",
            result.err()
        );

        let semantic = result.unwrap();
        assert_eq!(
            semantic.types.len(),
            2,
            "Both Department and Employee types should be present"
        );
    }

    #[test]
    fn test_type_consolidation_qualified_name_uniqueness() {
        // Regression: when input types `a::Foo` and `b::Foo` both conflict with
        // an output type `c::Foo`, all three must receive distinct names after
        // consolidation — no silent drops.
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let a_foo = Struct::new("a::Foo");
        let b_foo = Struct::new("b::Foo");
        let c_foo = Struct::new("c::Foo");

        schema.input_types.insert_type(a_foo.into());
        schema.input_types.insert_type(b_foo.into());
        schema.output_types.insert_type(c_foo.into());

        let stage = TypeConsolidationStage;
        stage.transform(&mut schema).unwrap();

        let type_names: Vec<String> = schema
            .input_types
            .types()
            .map(|t| t.name().to_string())
            .collect();

        // All three should be present with distinct names
        assert_eq!(
            type_names.len(),
            3,
            "All three Foo types should survive consolidation, got: {type_names:?}"
        );

        // Verify uniqueness — no two names are the same
        let unique_names: std::collections::HashSet<&String> = type_names.iter().collect();
        assert_eq!(
            unique_names.len(),
            3,
            "All three names should be distinct, got: {type_names:?}"
        );

        // Verify the naming convention: input types get "input." prefix,
        // output types get "output." prefix
        let has_input_a = type_names
            .iter()
            .any(|n| n.contains("input") && n.contains("a"));
        let has_input_b = type_names
            .iter()
            .any(|n| n.contains("input") && n.contains("b"));
        let has_output_c = type_names
            .iter()
            .any(|n| n.contains("output") && n.contains("c"));
        assert!(
            has_input_a,
            "Expected an input.a.Foo variant, got: {type_names:?}"
        );
        assert!(
            has_input_b,
            "Expected an input.b.Foo variant, got: {type_names:?}"
        );
        assert!(
            has_output_c,
            "Expected an output.c.Foo variant, got: {type_names:?}"
        );
    }

    #[test]
    fn test_resolve_types_does_not_confuse_variant_with_type() {
        // Regression: the resolve_types phase should resolve a function's type
        // reference "Status" to the Struct named "Status", not to an enum variant
        // that happens to also be named "Status".
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // A struct named "Status"
        let status_struct = Struct::new("Status");
        schema.input_types.insert_type(status_struct.into());

        // An enum with a variant named "Status"
        let mut state_enum = Enum::new("State".into());
        state_enum.variants = vec![Variant::new("Status".into()), Variant::new("Error".into())];
        schema.input_types.insert_type(state_enum.into());

        // A function that references "Status" — should resolve to the Struct
        let mut function = Function::new("get_status".into());
        function.input_type = Some(TypeReference::new("Status", vec![]));
        schema.functions.push(function);

        let normalizer = Normalizer::new();
        let result = normalizer.normalize(&schema);
        assert!(
            result.is_ok(),
            "Normalization should succeed: {:?}",
            result.err()
        );

        let semantic = result.unwrap();
        let func = semantic.functions.values().next().unwrap();

        // The function's input_type should resolve to the Status struct's ID
        let resolved_id = func
            .input_type
            .as_ref()
            .expect("input_type should be resolved");

        // It should be a Struct kind, not a Variant kind
        assert_eq!(
            resolved_id.kind,
            crate::SymbolKind::Struct,
            "Function's input_type should resolve to a Struct, not a Variant. Got: {resolved_id:?}"
        );
    }

    #[test]
    fn test_generate_unique_name_same_inner_module() {
        // Regression: two types with the same inner module and type name but
        // different outer modules must produce different unique names.
        let name_a = generate_unique_name("services::user::Profile");
        let name_b = generate_unique_name("auth::user::Profile");

        assert_ne!(
            name_a, name_b,
            "services::user::Profile and auth::user::Profile must produce different names, \
             got '{name_a}' and '{name_b}'"
        );

        // Verify they follow the expected PascalCase convention
        assert!(
            name_a.contains("Services") || name_a.contains("services"),
            "Expected 'services' component in name, got '{name_a}'"
        );
        assert!(
            name_b.contains("Auth") || name_b.contains("auth"),
            "Expected 'auth' component in name, got '{name_b}'"
        );
    }

    #[test]
    fn test_function_symbol_path_matches_id() {
        // Regression: after normalization, a function's SymbolId should be
        // retrievable from the symbol table via its path.
        let mut schema = Schema::new();
        schema.name = "API".to_string();

        let mut function = Function::new("get_user".into());
        function.input_type = None;
        function.output_type = None;
        schema.functions.push(function);

        let normalizer = Normalizer::new();
        let semantic = normalizer
            .normalize(&schema)
            .expect("Normalization should succeed");

        // Get the function's ID
        let (function_id, _) = semantic.functions.iter().next().unwrap();

        // Verify the symbol table can find it by path
        let found = semantic.symbol_table.get_by_path(&function_id.path);
        assert!(
            found.is_some(),
            "symbol_table.get_by_path({:?}) should return Some, but got None. \
             Function ID: {function_id:?}",
            function_id.path
        );

        let symbol_info = found.unwrap();
        assert_eq!(
            symbol_info.kind,
            crate::SymbolKind::Endpoint,
            "Symbol should be an Endpoint, got {:?}",
            symbol_info.kind
        );
    }
}
