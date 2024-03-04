#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Schema {
    pub name: String,
    pub types: Vec<Type>,
    pub _debug: String,
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            name: String::new(),
            types: Vec::new(),
            _debug: String::new(),
        }
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Type {
    pub name: String,
    pub fields: Vec<Field>,
    pub _debug: String,
}

impl Type {
    pub fn new(name: String) -> Self {
        Type {
            name,
            fields: Vec::new(),
            _debug: String::new(),
        }
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub _debug: String,
}

impl Field {
    pub fn new(name: String, ty: TypeRef) -> Self {
        Field {
            name,
            type_ref: ty,
            _debug: String::new(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub name: String,
}

impl TypeRef {
    pub fn new(name: String) -> Self {
        TypeRef { name }
    }
}

impl Default for TypeRef {
    fn default() -> Self {
        TypeRef {
            name: String::new(),
        }
    }
}
