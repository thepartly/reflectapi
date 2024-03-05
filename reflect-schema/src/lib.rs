use std::{collections::HashMap, fmt::Display};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub name: String,
    pub types: Vec<Type>,
    pub _debug: String,

    #[serde(skip_serializing, default)]
    types_map: HashMap<String, usize>,
}

impl Display for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_json().as_str())
    }
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            name: String::new(),
            types: Vec::new(),
            types_map: HashMap::new(),
            _debug: String::new(),
        }
    }

    pub fn types(&self) -> std::slice::Iter<Type> {
        self.types.iter()
    }

    pub fn has_type(&self, name: &str) -> bool {
        self.types_map.contains_key(name)
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        let Some(&index) = self.types_map.get(name) else {
            return None;
        };
        if index == usize::MAX {
            return None;
        }
        Some(&self.types[index])
    }

    pub fn reserve_type(&mut self, name: String) {
        self.types_map.insert(name, usize::MAX);
    }

    pub fn insert_type(&mut self, ty: Type) {
        self.types_map.insert(ty.name.clone(), self.types.len());
        self.types.push(ty);
    }

    pub fn from_json(json: &str) -> Self {
        let result: Self = serde_json::from_str(json).unwrap();
        let mut types_map = HashMap::new();
        for (i, ty) in result.types().enumerate() {
            types_map.insert(ty.name.clone(), i);
        }
        result
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

    pub fn get_type_refs(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|field| !field.type_ref.is_standard_type())
            .map(|field| field.type_ref.name.clone())
            .collect()
    }

    pub fn remap_type_refs(&mut self, remap: &std::collections::HashMap<String, String>) {
        for field in self.fields.iter_mut() {
            if field.type_ref.is_standard_type() {
                // TODO need to remap standard types, like:
                // better to do it on TypeRef creation, so these are allways full path
                // "String" -> "std::string::String"
                // "Vec" -> "std::collections::Vec"
                continue;
            }
            if let Some(new_path) = remap.get(&field.type_ref.name) {
                field.type_ref.name = new_path.clone();
            }
        }
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

    pub fn is_standard_type(&self) -> bool {
        // TODO add more standard types here
        const STANDARD_TYPES: [&str; 4] = ["String", "Vec", "i32", "u32"];
        // TODO optimize with static hashset
        STANDARD_TYPES.contains(&self.name.as_str())
    }
}

impl Default for TypeRef {
    fn default() -> Self {
        TypeRef {
            name: String::new(),
        }
    }
}
