use quote::ToTokens;
use std::cell::RefCell;
use std::fmt::Display;
use std::thread;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum ReflectType {
    Input,
    Output,
}

impl Default for ReflectType {
    fn default() -> Self {
        ReflectType::Input
    }
}

/// A type to collect errors together and format them.
///
/// Dropping this object will cause a panic. It must be consumed using `check`.
///
/// References can be shared since this type uses run-time exclusive mut checking.
#[derive(Default)]
pub(crate) struct Context {
    /// Purpose of a reflection: for input or output type
    reflect_type: ReflectType,

    pub encountered_type_refs: RefCell<std::collections::HashMap<String, syn::Type>>,

    // The contents will be set to `None` during checking. This is so that checking can be
    // enforced.
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl Context {
    /// Create a new context object.
    ///
    /// This object contains no errors, but will still trigger a panic if it is not `check`ed.
    pub fn new(reflect_type: ReflectType) -> Self {
        Context {
            reflect_type,
            encountered_type_refs: RefCell::new(std::collections::HashMap::new()),
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    pub fn reflect_type(&self) -> ReflectType {
        self.reflect_type
    }

    /// Add an error to the context object with a tokenenizable object.
    ///
    /// The object is used for spanning in error messages.
    pub fn impl_error<A: ToTokens, T: Display>(&self, obj: A, msg: T) {
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            // Curb monomorphization from generating too many identical methods.
            .push(syn::Error::new_spanned(obj.into_token_stream(), msg));
    }

    /// Add one of Syn's parse errors.
    pub fn syn_error(&self, err: syn::Error) {
        self.errors.borrow_mut().as_mut().unwrap().push(err);
    }

    /// Consume this object, producing a formatted error string if there are errors.
    pub fn check(self) -> syn::Result<std::collections::HashMap<String, syn::Type>> {
        let mut errors = self.errors.borrow_mut().take().unwrap().into_iter();

        let mut combined = match errors.next() {
            Some(first) => first,
            None => {
                return Ok(self
                    .encountered_type_refs
                    .replace(std::collections::HashMap::new()))
            }
        };

        for rest in errors {
            combined.combine(rest);
        }

        Err(combined)
    }

    /// Add a type reference actual type definition.
    pub fn encountered_type_ref(&self, name: String, ty: syn::Type) {
        self.encountered_type_refs.borrow_mut().insert(name, ty);
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !thread::panicking() && self.errors.borrow().is_some() {
            panic!("forgot to check for errors");
        }
    }
}
