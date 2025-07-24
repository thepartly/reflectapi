use core::fmt;

#[derive(Debug, Clone, Copy)]
pub struct StatusCode {}

impl core::fmt::Display for StatusCode {
    fn fmt(&self, _f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unimplemented!()
    }
}

impl StatusCode {
    pub const BAD_REQUEST: StatusCode = StatusCode {};

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

#[derive(Debug, Clone)]
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
}

pub struct HeaderName(());

impl HeaderName {
    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, InvalidHeaderName> {
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

#[derive(Debug, Clone)]
pub struct HeaderValue(());

impl HeaderValue {
    pub fn from_str(_s: &str) -> Result<Self, InvalidHeaderValue> {
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
