use reflectapi_schema::SemanticSchema;
use super::naming::NamingConvention;

/// Python Semantic IR for ReflectAPI
///
/// This module provides Python-specific semantic representations that translate
/// schema concepts into high-level Python modeling decisions. It answers the
/// question: "What is the most idiomatic way to model this concept in Python?"
///
/// Key responsibilities:
/// - Decide that internally tagged enums become DiscriminatedUnions  
/// - Decide that externally tagged enums become RootModelWrappers
/// - Identify functions requiring pagination patterns
/// - Determine factory pattern requirements
/// - Apply unknown-variant fallback policies
use std::collections::BTreeMap;

/// High-level Python modeling decisions for semantic concepts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PySemanticType {
    /// Simple Pydantic model with fields
    SimpleModel(ModelDef),

    /// Internally tagged enum -> discriminated union with literal discriminator
    DiscriminatedUnion(DiscriminatedUnionDef),

    /// Externally tagged enum -> RootModel wrapper with custom validation
    RootModelWrapper(RootModelDef),

    /// Generic type requiring runtime builder with .pyi stubs
    GenericRootModel(GenericRootModelDef),

    /// Complex enum patterns requiring factory methods
    FactoryPattern(FactoryDef),

    /// Function endpoint with pagination support
    PaginatableEndpoint(EndpointDef),

    /// Primitive type alias or newtype wrapper
    TypeAlias(TypeAliasDef),
}

/// Simple Pydantic BaseModel with typed fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDef {
    pub name: String,
    pub description: String,
    pub fields: BTreeMap<String, FieldDef>,
    pub generic_params: Vec<String>,
    pub base_classes: Vec<String>,
    pub decorator_config: DecoratorConfig,
}

/// Internally tagged discriminated union with literal type discrimination
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscriminatedUnionDef {
    pub name: String,
    pub description: String,
    pub discriminator_field: String,
    pub variants: BTreeMap<String, PySemanticType>, // Maps tag -> variant model
    pub unknown_variant_policy: UnknownPolicy,
    pub base_classes: Vec<String>,
}

/// Externally tagged enum as RootModel with custom validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootModelDef {
    pub name: String,
    pub description: String,
    pub union_type: String,          // The Union[...] type annotation
    pub variant_models: Vec<String>, // Names of variant model classes
    pub validation_strategy: ValidationStrategy,
    pub serialization_strategy: SerializationStrategy,
    pub requires_factory: bool,
}

/// Generic type requiring .py/.pyi split for static typing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericRootModelDef {
    pub name: String,
    pub description: String,
    pub type_params: Vec<String>,
    pub requires_runtime_builder: bool,
    pub requires_pyi_stub: bool,
    pub runtime_spec: RuntimeSpec,
}

/// Factory pattern for complex enum construction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryDef {
    pub name: String,
    pub target_type: String,
    pub static_methods: Vec<StaticMethodDef>,
    pub constants: Vec<ConstantDef>,
}

/// API endpoint with optional pagination logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDef {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub error_type: Option<String>,
    pub pagination: Option<PaginationPattern>,
    pub client_method_name: String,
}

/// Type alias or newtype wrapper
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasDef {
    pub name: String,
    pub target_type: String,
    pub description: String,
    pub requires_validation: bool,
}

/// Field definition with Python-specific typing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub python_name: String, // Sanitized for Python keywords
    pub type_annotation: String,
    pub description: String,
    pub deprecation_note: Option<String>,
    pub optional: bool,
    pub default_value: Option<String>,
    pub field_config: Option<FieldConfig>,
}

/// Static factory method for complex construction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticMethodDef {
    pub name: String,
    pub parameters: Vec<ParameterDef>,
    pub return_type: String,
    pub body_template: String,
}

/// Factory constant (for unit variants)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDef {
    pub name: String,
    pub value: String,
    pub type_annotation: String,
}

/// Function parameter with Python typing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDef {
    pub name: String,
    pub type_annotation: String,
    pub default_value: Option<String>,
}

/// Pydantic decorator configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratorConfig {
    pub config_dict: Option<String>,
    pub field_validators: Vec<String>,
    pub model_validators: Vec<String>,
}

/// Pydantic Field() configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldConfig {
    pub alias: Option<String>,
    pub discriminator: Option<String>,
    pub description: Option<String>,
    pub deprecated: Option<String>,
}

/// Strategy for validating externally tagged enums
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationStrategy {
    /// Use discriminator field to determine variant
    DiscriminatorBased { field: String },

    /// Try variants in order until one matches
    OrderedUnion { precedence: Vec<String> },

    /// Custom validation logic for complex cases
    CustomValidator { logic: ValidationLogic },
}

/// Strategy for serializing externally tagged enums
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationStrategy {
    /// Wrap in dict with variant name as key
    WrappedDict,

    /// Use raw variant value
    Direct,

    /// Custom serialization logic
    Custom { logic: SerializationLogic },
}

/// Custom validation implementation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationLogic {
    pub method_name: String,
    pub implementation: String,
}

/// Custom serialization implementation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializationLogic {
    pub method_name: String,
    pub implementation: String,
}

/// Runtime specification for generic types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpec {
    pub builder_spec: BTreeMap<String, String>,
    pub type_param_mapping: BTreeMap<String, String>,
}

/// Policy for handling unknown enum variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownPolicy {
    /// Raise ValueError on unknown variants
    Strict,

    /// Allow unknown variants with fallback type
    Fallback { fallback_type: String },

    /// Log warning and use default value
    Warn { default_value: String },
}

/// HTTP method for endpoint generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Pagination pattern detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginationPattern {
    /// Offset/limit pagination
    OffsetBased {
        offset_param: String,
        limit_param: String,
    },

    /// Cursor-based pagination  
    CursorBased {
        cursor_param: String,
        page_size_param: Option<String>,
    },

    /// Page number pagination
    PageBased {
        page_param: String,
        size_param: String,
    },
}

/// Python semantic IR containing high-level modeling decisions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PySemanticIR {
    pub package_name: String,
    pub description: String,

    /// All type definitions in topological order (respecting dependencies)
    pub types: Vec<(String, PySemanticType)>,

    /// Client endpoints grouped by service
    pub endpoints: BTreeMap<String, Vec<EndpointDef>>,

    /// Module-level imports required
    pub required_imports: RequiredImports,

    /// Global configuration
    pub config: PyCodegenConfig,
}

/// Import requirements computed from semantic analysis
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequiredImports {
    pub has_datetime: bool,
    pub has_date: bool,
    pub has_uuid: bool,
    pub has_enums: bool,
    pub has_literal: bool,
    pub has_annotated: bool,
    pub has_discriminated_unions: bool,
    pub has_externally_tagged_enums: bool,
    pub has_async: bool,
    pub has_sync: bool,
    pub has_testing: bool,
    pub has_reflectapi_option: bool,
    pub has_reflectapi_empty: bool,
    pub has_reflectapi_infallible: bool,
}

/// Python-specific code generation configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyCodegenConfig {
    pub generate_async: bool,
    pub generate_sync: bool,
    pub generate_testing: bool,
    pub generate_pyi_stubs: bool,
    pub base_url: Option<String>,
    pub package_name: String,
}

impl Default for PyCodegenConfig {
    fn default() -> Self {
        Self {
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            generate_pyi_stubs: false,
            base_url: None,
            package_name: "api_client".to_string(),
        }
    }
}

/// Semantic transformation from normalized schema to Python semantic decisions
pub struct PySemanticTransform {
    config: PyCodegenConfig,
    imports: RequiredImports,
    naming: NamingConvention,
}

impl PySemanticTransform {
    pub fn new(config: PyCodegenConfig) -> Self {
        Self {
            config,
            imports: RequiredImports::default(),
            naming: NamingConvention::new(),
        }
    }

    /// Transform normalized semantic schema into Python semantic IR
    pub fn transform(&mut self, schema: SemanticSchema) -> PySemanticIR {
        let mut types = Vec::new();
        let mut endpoints = BTreeMap::new();

        // Transform types in topological order (handle resolved cycles gracefully)
        let sorted_symbols = match schema.symbol_table.topological_sort() {
            Ok(sorted) => sorted,
            Err(cycle) => {
                // Cycles may still exist in dependency graph after resolution (e.g., due to boxing)
                eprintln!("Warning: Cycles detected in semantic schema, processing in arbitrary order: {:?}", cycle);
                schema.types.keys().cloned().collect()
            }
        };

        for symbol_id in sorted_symbols {
            if let Some(semantic_type) = schema.types.get(&symbol_id) {
                match semantic_type {
                    reflectapi_schema::SemanticType::Enum(e) => {
                        let (py_type, additional_types) = self.transform_enum(e, &schema);
                        
                        // Insert additional variant types FIRST (definition-before-use)
                        for (name, variant_type) in additional_types {
                            types.push((name, variant_type));
                        }
                        
                        // Then insert the main enum type
                        types.push((semantic_type.name().to_string(), py_type));
                    }
                    _ => {
                        let py_type = self.transform_type(semantic_type, &schema);
                        types.push((semantic_type.name().to_string(), py_type));
                    }
                }
            }
        }

        // Transform functions into endpoints
        for function in schema.functions.values() {
            let endpoint = self.transform_function(function.clone(), &schema);
            endpoints
                .entry("api".to_string())
                .or_insert_with(Vec::new)
                .push(endpoint);
        }

        PySemanticIR {
            package_name: self.config.package_name.clone(),
            description: schema.description,
            types,
            endpoints,
            required_imports: self.imports.clone(),
            config: self.config.clone(),
        }
    }

    fn transform_type(
        &mut self,
        semantic_type: &reflectapi_schema::SemanticType,
        schema: &SemanticSchema,
    ) -> PySemanticType {
        use reflectapi_schema::SemanticType;

        match semantic_type {
            SemanticType::Struct(s) => self.transform_struct(s, schema),
            SemanticType::Enum(_) => {
                panic!("transform_type should not be called for enums - handle in transform method directly")
            },
            SemanticType::Primitive(p) => self.transform_primitive(p),
        }
    }

    fn transform_struct(
        &mut self,
        strukt: &reflectapi_schema::SemanticStruct,
        _schema: &SemanticSchema,
    ) -> PySemanticType {
        let mut fields = BTreeMap::new();

        for field in strukt.fields.values() {
            let field_def = FieldDef {
                name: field.name.clone(),
                python_name: sanitize_python_identifier(&field.name),
                type_annotation: self.resolve_type_annotation(&field.type_ref),
                description: field.description.clone(),
                deprecation_note: field.deprecation_note.clone(),
                optional: !field.required,
                default_value: if field.required {
                    None
                } else {
                    Some("None".to_string())
                },
                field_config: None,
            };
            fields.insert(field.name.clone(), field_def);
        }

        PySemanticType::SimpleModel(ModelDef {
            name: self.naming.format_class_name(&strukt.name),
            description: strukt.description.clone(),
            fields,
            generic_params: strukt.parameters.iter().map(|p| p.name.clone()).collect(),
            base_classes: vec!["BaseModel".to_string()],
            decorator_config: DecoratorConfig {
                config_dict: None,
                field_validators: vec![],
                model_validators: vec![],
            },
        })
    }

    fn transform_enum(
        &mut self,
        enm: &reflectapi_schema::SemanticEnum,
        _schema: &SemanticSchema,
    ) -> (PySemanticType, Vec<(String, PySemanticType)>) {
        use reflectapi_schema::Representation;

        match &enm.representation {
            Representation::Internal { tag } => {
                self.imports.has_discriminated_unions = true;
                self.imports.has_literal = true;
                self.imports.has_annotated = true;

                // Transform variants into Python semantic types
                let mut python_variants = BTreeMap::new();

                for variant in enm.variants.values() {
                    // For internally tagged enums, each variant becomes a model with the discriminator field
                    let mut variant_fields = BTreeMap::new();
                    
                    // Transform each field in the variant
                    for (_field_id, field) in variant.fields.iter() {
                        let field_def = FieldDef {
                            name: field.name.clone(),
                            python_name: sanitize_python_identifier(&field.name),
                            type_annotation: self.resolve_type_annotation(&field.type_ref),
                            description: field.description.clone(),
                            deprecation_note: field.deprecation_note.clone(),
                            optional: !field.required,
                            default_value: if field.required {
                                None
                            } else {
                                Some("None".to_string())
                            },
                            field_config: None,
                        };
                        variant_fields.insert(field.name.clone(), field_def);
                    }

                    let variant_class_name = self.naming.format_variant_class_name(&enm.name, &variant.name);
                    let variant_model = PySemanticType::SimpleModel(ModelDef {
                        name: variant_class_name,
                        description: variant.description.clone(),
                        fields: variant_fields,
                        generic_params: vec![],
                        base_classes: vec!["BaseModel".to_string()],
                        decorator_config: DecoratorConfig {
                            config_dict: None,
                            field_validators: vec![],
                            model_validators: vec![],
                        },
                    });

                    python_variants.insert(variant.serde_name.clone(), variant_model);
                }

                (PySemanticType::DiscriminatedUnion(DiscriminatedUnionDef {
                    name: self.naming.format_class_name(&enm.name),
                    description: enm.description.clone(),
                    discriminator_field: tag.clone(),
                    variants: python_variants,
                    unknown_variant_policy: UnknownPolicy::Strict,
                    base_classes: vec!["BaseModel".to_string()],
                }), vec![])
            }

            Representation::External => {
                self.imports.has_externally_tagged_enums = true;

                // Extract variant information for building union
                let mut variant_names = Vec::new();
                let mut variant_models = Vec::new();

                let mut additional_types = Vec::new();

                for variant in enm.variants.values() {
                    let variant_model_name = if variant.fields.is_empty() {
                        // Unit variant - use Literal
                        format!("Literal[\"{}\"]", variant.name)
                    } else {
                        // Complex variant - create dedicated model class
                        let variant_class_name = self.naming.format_variant_class_name(&enm.name, &variant.name);
                        
                        // Transform variant fields
                        let mut variant_fields = BTreeMap::new();
                        for (_field_id, field) in variant.fields.iter() {
                            let field_def = FieldDef {
                                name: field.name.clone(),
                                python_name: sanitize_python_identifier(&field.name),
                                type_annotation: self.resolve_type_annotation(&field.type_ref),
                                description: field.description.clone(),
                                deprecation_note: field.deprecation_note.clone(),
                                optional: !field.required,
                                default_value: if field.required {
                                    None
                                } else {
                                    Some("None".to_string())
                                },
                                field_config: None,
                            };
                            variant_fields.insert(field.name.clone(), field_def);
                        }
                        
                        let variant_model = PySemanticType::SimpleModel(ModelDef {
                            name: variant_class_name.clone(),
                            description: format!("Variant {} of {}", variant.name, enm.name),
                            fields: variant_fields,
                            generic_params: vec![],
                            base_classes: vec!["BaseModel".to_string()],
                            decorator_config: DecoratorConfig {
                                config_dict: None,
                                field_validators: vec![],
                                model_validators: vec![],
                            },
                        });
                        
                        // Collect the variant type to return
                        additional_types.push((variant_class_name.clone(), variant_model));
                        
                        variant_class_name
                    };

                    variant_names.push(variant.name.clone());
                    variant_models.push(variant_model_name);
                }

                let union_type = if variant_models.is_empty() {
                    "Any".to_string()
                } else {
                    format!("Union[{}]", variant_models.join(", "))
                };

                let main_type = PySemanticType::RootModelWrapper(RootModelDef {
                    name: self.naming.format_class_name(&enm.name),
                    description: enm.description.clone(),
                    union_type,
                    variant_models,
                    validation_strategy: ValidationStrategy::OrderedUnion {
                        precedence: variant_names,
                    },
                    serialization_strategy: SerializationStrategy::WrappedDict,
                    requires_factory: enm.variants.len() > 3, // Factory for complex enums
                });

                (main_type, additional_types)
            }

            _ => {
                // Adjacently tagged, untagged, etc.
                self.imports.has_externally_tagged_enums = true;

                // Extract variant information similar to external tagging
                let mut variant_names = Vec::new();
                let mut variant_models = Vec::new();

                let mut additional_types = Vec::new();

                for variant in enm.variants.values() {
                    let variant_model_name = if variant.fields.is_empty() {
                        format!("Literal[\"{}\"]", variant.name)
                    } else {
                        // Complex variant - create dedicated model class
                        let variant_class_name = self.naming.format_variant_class_name(&enm.name, &variant.name);
                        
                        // Transform variant fields
                        let mut variant_fields = BTreeMap::new();
                        for (_field_id, field) in variant.fields.iter() {
                            let field_def = FieldDef {
                                name: field.name.clone(),
                                python_name: sanitize_python_identifier(&field.name),
                                type_annotation: self.resolve_type_annotation(&field.type_ref),
                                description: field.description.clone(),
                                deprecation_note: field.deprecation_note.clone(),
                                optional: !field.required,
                                default_value: if field.required {
                                    None
                                } else {
                                    Some("None".to_string())
                                },
                                field_config: None,
                            };
                            variant_fields.insert(field.name.clone(), field_def);
                        }
                        
                        let variant_model = PySemanticType::SimpleModel(ModelDef {
                            name: variant_class_name.clone(),
                            description: format!("Variant {} of {}", variant.name, enm.name),
                            fields: variant_fields,
                            generic_params: vec![],
                            base_classes: vec!["BaseModel".to_string()],
                            decorator_config: DecoratorConfig {
                                config_dict: None,
                                field_validators: vec![],
                                model_validators: vec![],
                            },
                        });
                        
                        // Collect the variant type to return
                        additional_types.push((variant_class_name.clone(), variant_model));
                        
                        variant_class_name
                    };

                    variant_names.push(variant.name.clone());
                    variant_models.push(variant_model_name);
                }

                let union_type = if variant_models.is_empty() {
                    "Any".to_string()
                } else {
                    format!("Union[{}]", variant_models.join(", "))
                };

                let main_type = PySemanticType::RootModelWrapper(RootModelDef {
                    name: self.naming.format_class_name(&enm.name),
                    description: enm.description.clone(),
                    union_type,
                    variant_models,
                    validation_strategy: ValidationStrategy::CustomValidator {
                        logic: ValidationLogic {
                            method_name: "_validate".to_string(),
                            implementation: "return data".to_string(),
                        },
                    },
                    serialization_strategy: SerializationStrategy::Custom {
                        logic: SerializationLogic {
                            method_name: "_serialize".to_string(),
                            implementation: "return self.root".to_string(),
                        },
                    },
                    requires_factory: false,
                });

                (main_type, additional_types)
            }
        }
    }

    fn transform_primitive(
        &mut self,
        primitive: &reflectapi_schema::SemanticPrimitive,
    ) -> PySemanticType {
        // Most primitives map to built-in Python types
        PySemanticType::TypeAlias(TypeAliasDef {
            name: self.naming.format_class_name(&primitive.name),
            target_type: map_primitive_to_python(&primitive.name),
            description: primitive.description.clone(),
            requires_validation: false,
        })
    }

    fn transform_function(
        &mut self,
        function: reflectapi_schema::SemanticFunction,
        _schema: &SemanticSchema,
    ) -> EndpointDef {
        // Determine HTTP method from function name/path
        let method = if function.name.starts_with("get_") || function.readonly {
            HttpMethod::Get
        } else if function.name.starts_with("create_") || function.name.starts_with("post_") {
            HttpMethod::Post
        } else if function.name.starts_with("update_") || function.name.starts_with("put_") {
            HttpMethod::Put
        } else if function.name.starts_with("delete_") {
            HttpMethod::Delete
        } else {
            HttpMethod::Post // Default
        };

        EndpointDef {
            name: function.name.clone(),
            method,
            path: function.path.clone(),
            input_type: None,  // TODO: Resolve from SymbolId
            output_type: None, // TODO: Resolve from SymbolId
            error_type: None,  // TODO: Resolve from SymbolId
            pagination: None,  // TODO: Detect pagination patterns
            client_method_name: to_snake_case(&function.name),
        }
    }

    fn resolve_type_annotation(
        &mut self,
        type_ref: &reflectapi_schema::ResolvedTypeReference,
    ) -> String {
        // First resolve the base type
        let base_type = self.resolve_type_name(&type_ref.original_name);

        // Then handle generic arguments if present
        if type_ref.arguments.is_empty() {
            base_type
        } else {
            let arg_types: Vec<String> = type_ref
                .arguments
                .iter()
                .map(|arg| self.resolve_type_annotation(arg))
                .collect();

            // Handle special cases for common generic types
            match type_ref.original_name.as_str() {
                "std::vec::Vec" => {
                    format!("list[{}]", arg_types.join(", "))
                }
                "std::collections::HashMap" | "std::collections::BTreeMap" => {
                    if arg_types.len() >= 2 {
                        format!("dict[{}, {}]", arg_types[0], arg_types[1])
                    } else {
                        "dict".to_string()
                    }
                }
                "std::collections::HashSet" | "std::collections::BTreeSet" => {
                    format!("set[{}]", arg_types.join(", "))
                }
                "std::option::Option" => {
                    self.imports.has_literal = true; // For Optional import
                    format!("Optional[{}]", arg_types.join(", "))
                }
                "reflectapi::Option" => {
                    self.imports.has_reflectapi_option = true;
                    format!("ReflectapiOption[{}]", arg_types.join(", "))
                }
                _ => {
                    // Generic class with parameters
                    format!("{}[{}]", base_type, arg_types.join(", "))
                }
            }
        }
    }

    /// Resolve Rust type names to appropriate Python type annotations
    fn resolve_type_name(&mut self, type_name: &str) -> String {
        match type_name {
            // Standard Rust types -> Python types
            "std::string::String" => "str".to_string(),
            "std::vec::Vec" => "list".to_string(),
            "std::collections::HashMap" => "dict".to_string(),
            "std::collections::BTreeMap" => "dict".to_string(),
            "std::collections::HashSet" => "set".to_string(),
            "std::collections::BTreeSet" => "set".to_string(),

            // Option types -> Optional
            "std::option::Option" => {
                self.imports.has_literal = true; // For Optional import
                "Optional".to_string()
            }

            // Primitive types
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "int".to_string(),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "int".to_string(),
            "f32" | "f64" => "float".to_string(),
            "bool" => "bool".to_string(),
            "str" => "str".to_string(),
            "char" => "str".to_string(),

            // DateTime and Date types
            "chrono::DateTime" => {
                self.imports.has_datetime = true;
                "datetime.datetime".to_string()
            }
            "chrono::Date" => {
                self.imports.has_date = true;
                "datetime.date".to_string()
            }
            "chrono::NaiveDateTime" => {
                self.imports.has_datetime = true;
                "datetime.datetime".to_string()
            }

            // UUID type
            "uuid::Uuid" => {
                self.imports.has_uuid = true;
                "UUID".to_string()
            }

            // ReflectAPI types
            "reflectapi::Option" => {
                self.imports.has_reflectapi_option = true;
                "ReflectapiOption".to_string()
            }
            "reflectapi::Empty" => {
                self.imports.has_reflectapi_empty = true;
                "Empty".to_string()
            }
            "reflectapi::Infallible" => {
                self.imports.has_reflectapi_infallible = true;
                "Infallible".to_string()
            }

            // Tuple types
            name if name.starts_with("std::tuple::Tuple") => {
                // Extract tuple size and map accordingly
                if name == "std::tuple::Tuple0" {
                    "None".to_string() // Unit type
                } else {
                    "tuple".to_string() // Generic tuple
                }
            }

            // Box and reference types -> unwrap to inner type
            name if name.starts_with("std::boxed::Box<") => {
                // Extract inner type: Box<T> -> T
                if let Some(inner) = extract_generic_inner(name, "std::boxed::Box<") {
                    self.resolve_type_name(&inner)
                } else {
                    "Any".to_string()
                }
            }

            name if name.starts_with("std::sync::Arc<") => {
                // Extract inner type: Arc<T> -> T
                if let Some(inner) = extract_generic_inner(name, "std::sync::Arc<") {
                    self.resolve_type_name(&inner)
                } else {
                    "Any".to_string()
                }
            }

            // Generic types with parameters - handle later in syntax transform
            name if name.contains("::") => {
                // Clean up qualified names to just the type name
                let clean_name = name.split("::").last().unwrap_or(name).to_string();
                clean_name
            }

            // Module-prefixed types like "input.Option" or "output.Pet"
            name if name.contains('.') => {
                // Use centralized naming to ensure consistency
                self.naming.format_class_name(name)
            }

            // Default case - return the type name as-is
            _ => type_name.to_string(),
        }
    }
}

/// Extract the inner type from a generic wrapper like Box<T> or Arc<T>
fn extract_generic_inner(type_name: &str, prefix: &str) -> Option<String> {
    if type_name.starts_with(prefix) && type_name.ends_with('>') {
        let inner_start = prefix.len();
        let inner_end = type_name.len() - 1;
        if inner_start < inner_end {
            Some(type_name[inner_start..inner_end].to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Convert Python keywords and invalid identifiers to valid Python names
fn sanitize_python_identifier(name: &str) -> String {
    let cleaned = name
        .replace(['.', '-'], "_")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    const PYTHON_KEYWORDS: &[&str] = &[
        "and", "as", "assert", "break", "class", "continue", "def", "del", "elif", "else",
        "except", "exec", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
        "not", "or", "pass", "print", "raise", "return", "try", "while", "with", "yield", "async",
        "await", "nonlocal",
    ];

    let mut result = if PYTHON_KEYWORDS.contains(&cleaned.as_str()) {
        format!("{}_", cleaned)
    } else {
        cleaned
    };

    if result.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        result = format!("field_{}", result);
    }

    result
}

/// Convert CamelCase to snake_case
fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        match c {
            // Convert separators to underscores
            '.' | '-' | ' ' => {
                if !result.is_empty() && !result.ends_with('_') {
                    result.push('_');
                }
            }
            // Handle camelCase
            _ if c.is_uppercase() && i > 0 => {
                if !result.ends_with('_') {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap_or(c));
            }
            // Regular characters
            _ => {
                result.push(c.to_lowercase().next().unwrap_or(c));
            }
        }
    }
    result
}

/// Map primitive type names to Python types
fn map_primitive_to_python(name: &str) -> String {
    match name {
        "String" | "str" => "str",
        "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => "int",
        "f32" | "f64" => "float",
        "bool" => "bool",
        "Vec" => "list",
        "HashMap" => "dict",
        "Option" => "Optional",
        _ => "Any",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_python_identifier() {
        assert_eq!(sanitize_python_identifier("class"), "class_");
        assert_eq!(sanitize_python_identifier("normal_field"), "normal_field");
        assert_eq!(sanitize_python_identifier("123field"), "field_123field");
        assert_eq!(sanitize_python_identifier("field-name"), "field_name");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("getUserById"), "get_user_by_id");
        assert_eq!(to_snake_case("createUser"), "create_user");
        assert_eq!(to_snake_case("simplefunction"), "simplefunction");
    }
}
