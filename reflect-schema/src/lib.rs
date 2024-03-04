#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub types: Vec<Type>,
    pub _debug: String,
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            types: Vec::new(),
            _debug: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: String,
    pub _debug: String,
}

impl Field {
    pub fn new(name: String, ty: String) -> Self {
        Field {
            name,
            ty,
            _debug: String::new(),
        }
    }
}
