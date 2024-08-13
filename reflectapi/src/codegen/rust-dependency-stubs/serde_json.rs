pub struct Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
    }
}

pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T, Error> {
    unimplemented!()
}

pub fn to_vec<T>(_value: &T) -> Result<Vec<u8>, Error> {
    unimplemented!()
}

pub fn to_value<T>(_value: &T) -> Result<Value, Error> {
    unimplemented!()
}

pub enum Value {
    Null,
    String(String),
    Object(std::collections::HashMap<String, Value>),
}

impl core::fmt::Display for Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
    }
}
