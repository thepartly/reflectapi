/// Python Syntax IR for ReflectAPI
///
/// This module provides Python-specific syntax tree representations that map
/// semantic decisions into concrete Python code structures. It is responsible
/// for the syntactic structure of Python code while remaining language-agnostic
/// in its semantic understanding.
///
/// Key responsibilities:
/// - Translate DiscriminatedUnion -> TypeAlias of Annotated[Union[...]]
/// - Translate RootModelWrapper -> Class inheriting from RootModel
/// - Translate FactoryPattern -> Class with static methods
/// - Handle import collection and deduplication
/// - Maintain deterministic ordering for stable output

/// Complete Python module representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub docstring: Option<String>,
    pub future_imports: Vec<String>,
    pub imports: ImportBlock,
    pub items: Vec<Item>,
    pub exports: Option<Vec<String>>,
}

/// Organized import block with proper grouping
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportBlock {
    pub standard: Vec<Import>,
    pub third_party: Vec<Import>,
    pub local: Vec<Import>,
}

/// Python import statement
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Import {
    /// import module [as alias]
    Simple {
        module: String,
        alias: Option<String>,
    },

    /// from module import name1 [as alias1], name2 [as alias2], ...
    From {
        module: String,
        names: Vec<ImportName>,
    },
}

/// Named import with optional alias
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

/// Top-level module item
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Class(Class),
    Function(Function),
    TypeAlias(TypeAlias),
    Constant(Constant),
    Assignment(Assignment),
    Expression(Expression),
}

/// Python class definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub bases: Vec<TypeExpr>,
    pub decorators: Vec<Decorator>,
    pub docstring: Option<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
    pub nested_classes: Vec<Class>,
    pub meta: ClassMeta,
}

/// Python class metadata
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassMeta {
    pub is_generic: bool,
    pub type_params: Vec<String>,
    pub is_dataclass: bool,
    pub is_abstract: bool,
}

/// Python function definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub args: Vec<Argument>,
    pub return_type: Option<TypeExpr>,
    pub docstring: Option<String>,
    pub body: Vec<Statement>,
    pub is_async: bool,
    pub is_static: bool,
    pub is_classmethod: bool,
}

/// Function argument with optional defaults and type annotations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub default: Option<Expression>,
    pub kind: ArgumentKind,
}

/// Argument binding type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentKind {
    Positional,
    PositionalOnly,
    KeywordOnly,
    VarArgs,   // *args
    VarKwargs, // **kwargs
}

/// Python class field (for type annotations)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub type_annotation: TypeExpr,
    pub default: Option<Expression>,
    pub field_config: Option<Expression>, // Field(...) configuration
}

/// Type alias definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub name: String,
    pub type_params: Vec<String>,
    pub value: TypeExpr,
}

/// Module-level constant
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub name: String,
    pub value: Expression,
    pub type_annotation: Option<TypeExpr>,
}

/// Variable assignment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub target: String,
    pub value: Expression,
    pub type_annotation: Option<TypeExpr>,
}

/// Python decorator
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<Expression>,
    pub kwargs: Vec<(String, Expression)>,
}

/// Python type expressions for annotations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    /// Simple name reference (str, int, MyClass)
    Name(String),

    /// Generic type with arguments (List[str], Dict[str, int])
    Subscript {
        base: Box<TypeExpr>,
        args: Vec<TypeExpr>,
    },

    /// Union type (str | int, Union[str, int])
    Union(Vec<TypeExpr>),

    /// Optional type (str | None)
    Optional(Box<TypeExpr>),

    /// Literal type (Literal["value"])
    Literal(LiteralValue),

    /// Tuple type (tuple[str, int])
    Tuple(Vec<TypeExpr>),

    /// Callable type (Callable[[str], int])
    Callable {
        args: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
    },

    /// Type variable (T, K, V)
    TypeVar(String),

    /// Annotated type (Annotated[str, Field(...)])
    Annotated {
        base: Box<TypeExpr>,
        metadata: Vec<Expression>,
    },

    /// String literal type annotation (for forward references)
    ForwardRef(String),
}

/// Python literal values
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}

impl Eq for LiteralValue {}

impl PartialOrd for LiteralValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LiteralValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (LiteralValue::None, LiteralValue::None) => Ordering::Equal,
            (LiteralValue::None, _) => Ordering::Less,
            (_, LiteralValue::None) => Ordering::Greater,
            (LiteralValue::Bool(a), LiteralValue::Bool(b)) => a.cmp(b),
            (LiteralValue::Bool(_), _) => Ordering::Less,
            (_, LiteralValue::Bool(_)) => Ordering::Greater,
            (LiteralValue::Int(a), LiteralValue::Int(b)) => a.cmp(b),
            (LiteralValue::Int(_), _) => Ordering::Less,
            (_, LiteralValue::Int(_)) => Ordering::Greater,
            (LiteralValue::Float(a), LiteralValue::Float(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (LiteralValue::Float(_), _) => Ordering::Less,
            (_, LiteralValue::Float(_)) => Ordering::Greater,
            (LiteralValue::String(a), LiteralValue::String(b)) => a.cmp(b),
            (LiteralValue::String(_), _) => Ordering::Less,
            (_, LiteralValue::String(_)) => Ordering::Greater,
            (LiteralValue::Bytes(a), LiteralValue::Bytes(b)) => a.cmp(b),
        }
    }
}

/// Python expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    /// Variable name reference
    Name(String),

    /// Literal value
    Literal(LiteralValue),

    /// Function call
    Call {
        func: Box<Expression>,
        args: Vec<Expression>,
        kwargs: Vec<(String, Expression)>,
    },

    /// Attribute access (obj.attr)
    Attribute {
        value: Box<Expression>,
        attr: String,
    },

    /// Subscript access (obj[key])
    Subscript {
        value: Box<Expression>,
        slice: Box<Expression>,
    },

    /// Binary operation (a + b)
    BinOp {
        left: Box<Expression>,
        op: BinOperator,
        right: Box<Expression>,
    },

    /// Unary operation (-x, not x)
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },

    /// Comparison (a == b, x in y)
    Compare {
        left: Box<Expression>,
        ops: Vec<CompareOp>,
        comparators: Vec<Expression>,
    },

    /// Dictionary literal ({key: value})
    Dict {
        keys: Vec<Expression>,
        values: Vec<Expression>,
    },

    /// List literal ([1, 2, 3])
    List(Vec<Expression>),

    /// Tuple literal ((1, 2, 3))
    Tuple(Vec<Expression>),

    /// Set literal ({1, 2, 3})
    Set(Vec<Expression>),

    /// Lambda function (lambda x: x + 1)
    Lambda {
        args: Vec<String>,
        body: Box<Expression>,
    },

    /// Conditional expression (x if condition else y)
    IfExp {
        test: Box<Expression>,
        body: Box<Expression>,
        orelse: Box<Expression>,
    },

    /// List comprehension ([x for x in items])
    ListComp {
        element: Box<Expression>,
        generators: Vec<Comprehension>,
    },

    /// F-string (f"Hello {name}")
    FormattedString { parts: Vec<FormattedStringPart> },
}

/// Python statements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Expression statement
    Expression(Expression),

    /// Assignment (x = y)
    Assignment { target: String, value: Expression },

    /// Annotated assignment (x: int = 5)
    AnnAssignment {
        target: String,
        annotation: TypeExpr,
        value: Option<Expression>,
    },

    /// Return statement
    Return(Option<Expression>),

    /// If statement
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },

    /// For loop
    For {
        target: String,
        iter: Expression,
        body: Vec<Statement>,
    },

    /// While loop
    While {
        condition: Expression,
        body: Vec<Statement>,
    },

    /// Try/except block
    Try {
        body: Vec<Statement>,
        handlers: Vec<ExceptionHandler>,
        orelse: Option<Vec<Statement>>,
        finally_body: Option<Vec<Statement>>,
    },

    /// Raise exception
    Raise(Option<Expression>),

    /// Pass statement
    Pass,

    /// Continue statement
    Continue,

    /// Break statement
    Break,

    /// Assert statement
    Assert {
        test: Expression,
        msg: Option<Expression>,
    },
}

/// Exception handler for try/except
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionHandler {
    pub exception_type: Option<TypeExpr>,
    pub name: Option<String>,
    pub body: Vec<Statement>,
}

/// List comprehension generator
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comprehension {
    pub target: String,
    pub iter: Expression,
    pub conditions: Vec<Expression>,
}

/// F-string formatting parts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormattedStringPart {
    String(String),
    Expression {
        expr: Expression,
        format_spec: Option<String>,
    },
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOperator {
    Add,
    Sub,
    Mult,
    Div,
    FloorDiv,
    Mod,
    Pow,
    BitOr,
    BitXor,
    BitAnd,
    LShift,
    RShift,
    MatMult,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
    Invert,
    Not,
    UAdd,
    USub,
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
}

impl Module {
    pub fn new(name: String) -> Self {
        Self {
            name,
            docstring: None,
            future_imports: Vec::new(),
            imports: ImportBlock::default(),
            items: Vec::new(),
            exports: None,
        }
    }

    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn add_import(&mut self, import: Import, category: ImportCategory) {
        match category {
            ImportCategory::Standard => self.imports.standard.push(import),
            ImportCategory::ThirdParty => self.imports.third_party.push(import),
            ImportCategory::Local => self.imports.local.push(import),
        }
    }
}

/// Category for import organization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportCategory {
    Standard,
    ThirdParty,
    Local,
}

impl Class {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bases: Vec::new(),
            decorators: Vec::new(),
            docstring: None,
            fields: Vec::new(),
            methods: Vec::new(),
            nested_classes: Vec::new(),
            meta: ClassMeta::default(),
        }
    }

    pub fn add_base(&mut self, base: TypeExpr) {
        self.bases.push(base);
    }

    pub fn add_decorator(&mut self, decorator: Decorator) {
        self.decorators.push(decorator);
    }

    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    pub fn add_method(&mut self, method: Function) {
        self.methods.push(method);
    }
}

impl Function {
    pub fn new(name: String) -> Self {
        Self {
            name,
            decorators: Vec::new(),
            args: Vec::new(),
            return_type: None,
            docstring: None,
            body: Vec::new(),
            is_async: false,
            is_static: false,
            is_classmethod: false,
        }
    }

    pub fn add_arg(&mut self, arg: Argument) {
        self.args.push(arg);
    }

    pub fn add_statement(&mut self, stmt: Statement) {
        self.body.push(stmt);
    }

    pub fn add_decorator(&mut self, decorator: Decorator) {
        self.decorators.push(decorator);
    }
}

impl TypeExpr {
    /// Create a simple name type expression
    pub fn name(name: &str) -> Self {
        TypeExpr::Name(name.to_string())
    }

    /// Create a generic type expression (List[str])
    pub fn generic(base: &str, args: Vec<TypeExpr>) -> Self {
        TypeExpr::Subscript {
            base: Box::new(TypeExpr::name(base)),
            args,
        }
    }

    /// Create an optional type (T | None)
    pub fn optional(inner: TypeExpr) -> Self {
        TypeExpr::Optional(Box::new(inner))
    }

    /// Create a union type
    pub fn union(types: Vec<TypeExpr>) -> Self {
        if types.len() == 1 {
            types.into_iter().next().unwrap()
        } else {
            TypeExpr::Union(types)
        }
    }
}

impl Expression {
    /// Create a name expression
    pub fn name(name: &str) -> Self {
        Expression::Name(name.to_string())
    }

    /// Create a string literal
    pub fn string(value: &str) -> Self {
        Expression::Literal(LiteralValue::String(value.to_string()))
    }

    /// Create a function call
    pub fn call(func: Expression, args: Vec<Expression>) -> Self {
        Expression::Call {
            func: Box::new(func),
            args,
            kwargs: Vec::new(),
        }
    }

    /// Create attribute access
    pub fn attr(value: Expression, attr: &str) -> Self {
        Expression::Attribute {
            value: Box::new(value),
            attr: attr.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let mut module = Module::new("test_module".to_string());
        module.docstring = Some("Test module".to_string());

        let class = Class::new("TestClass".to_string());
        module.add_item(Item::Class(class));

        assert_eq!(module.name, "test_module");
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_type_expr_creation() {
        let str_type = TypeExpr::name("str");
        let list_str = TypeExpr::generic("List", vec![str_type.clone()]);
        let optional_str = TypeExpr::optional(str_type);

        match list_str {
            TypeExpr::Subscript { base, args } => {
                assert_eq!(*base, TypeExpr::name("List"));
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected Subscript"),
        }

        match optional_str {
            TypeExpr::Optional(inner) => {
                assert_eq!(*inner, TypeExpr::name("str"));
            }
            _ => panic!("Expected Optional"),
        }
    }

    #[test]
    fn test_class_building() {
        let mut class = Class::new("User".to_string());
        class.add_base(TypeExpr::name("BaseModel"));

        let field = Field {
            name: "name".to_string(),
            type_annotation: TypeExpr::name("str"),
            default: None,
            field_config: None,
        };
        class.add_field(field);

        assert_eq!(class.bases.len(), 1);
        assert_eq!(class.fields.len(), 1);
        assert_eq!(class.fields[0].name, "name");
    }
}
