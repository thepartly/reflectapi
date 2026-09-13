use core::fmt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Method;

impl Method {
    pub const POST: Self = Self;
}

#[derive(Debug)]
pub struct StatusCode {}

impl core::fmt::Display for StatusCode {
    fn fmt(&self, _f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unimplemented!()
    }
}

impl StatusCode {
    pub const OK: Self = Self {};

    pub fn is_client_error(&self) -> bool {
        unimplemented!()
    }

    pub fn is_server_error(&self) -> bool {
        unimplemented!()
    }

    pub fn is_success(&self) -> bool {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub struct HeaderMap<T = HeaderValue> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> HeaderMap<T> {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn insert(&mut self, _name: HeaderName, _value: T) {
        unimplemented!()
    }

    pub fn contains_key(&self, _name: &HeaderName) -> bool {
        unimplemented!()
    }
}

impl<'a, T> IntoIterator for &'a HeaderMap<T> {
    type Item = (&'a HeaderName, &'a T);
    type IntoIter = std::vec::IntoIter<(&'a HeaderName, &'a T)>;

    fn into_iter(self) -> Self::IntoIter {
        unimplemented!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HeaderName(());

impl HeaderName {
    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, InvalidHeaderName> {
        unimplemented!()
    }

    pub fn from_static(_s: &'static str) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct InvalidHeaderName(());

impl fmt::Display for InvalidHeaderName {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl std::error::Error for InvalidHeaderName {}

pub mod header {
    pub static ACCEPT: super::HeaderName = super::HeaderName(());
}

#[derive(Clone, Debug)]
pub struct HeaderValue(());

impl HeaderValue {
    pub fn from_str(_s: &str) -> Result<Self, InvalidHeaderValue> {
        unimplemented!()
    }

    pub fn from_static(_s: &'static str) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct InvalidHeaderValue(());

impl fmt::Display for InvalidHeaderValue {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl std::error::Error for InvalidHeaderValue {}
