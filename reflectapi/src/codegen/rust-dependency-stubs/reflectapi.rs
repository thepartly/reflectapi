#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Option<T> {
    #[default]
    Undefined,
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Empty {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Infallible {}

pub mod rt;
