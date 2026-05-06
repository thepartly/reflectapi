pub struct Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
    }
}

pub fn from_slice<'a, T>(_v: &'a [u8]) -> Result<T, Error> {
    unimplemented!()
}

pub fn to_vec<T>(_value: &T) -> Result<Vec<u8>, Error> {
    unimplemented!()
}

pub fn to_string<T>(_value: &T) -> Result<String, Error> {
    unimplemented!()
}

pub fn to_value<T>(_value: &T) -> Result<Value, Error> {
    unimplemented!()
}

pub fn from_str<T>(_s: &str) -> Result<T, Error> {
    unimplemented!()
}

pub fn from_value<T>(_value: Value) -> Result<T, Error> {
    unimplemented!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    String(String),
    Object(std::collections::HashMap<String, Value>),
}

impl Value {
    pub fn get(&self, _key: &str) -> Option<&Value> {
        unimplemented!()
    }
}

impl core::fmt::Display for Value {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
    }
}
