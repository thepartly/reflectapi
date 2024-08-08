use quote::ToTokens;
use std::cell::RefCell;
use std::fmt::Display;
use std::thread;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
pub(crate) enum ReflectType {
    #[default]
    Input,
    Output,
}

pub(crate) struct ContextEncounters {
    pub fields: Vec<(reflectapi_schema::TypeReference, syn::Type)>,
    pub generics: Vec<(reflectapi_schema::TypeParameter, syn::Ident)>,
}

/// A type to collect errors together and format them.
///
/// Dropping this object will cause a panic. It must be consumed using `check`.
///
/// References can be shared since this type uses run-time exclusive mut checking.
#[derive(Default)]
pub(crate) struct Context {
    /// Purpose of a reflection: for input or output type
    reflectapi_type: ReflectType,

    encountered_type_fields: RefCell<Vec<(reflectapi_schema::TypeReference, syn::Type)>>,
    encountered_type_generics: RefCell<Vec<(reflectapi_schema::TypeParameter, syn::Ident)>>,

    // The contents will be set to `None` during checking. This is so that checking can be
    // enforced.
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl Context {
    /// Create a new context object.
    ///
    /// This object contains no errors, but will still trigger a panic if it is not `check`ed.
    pub fn new(reflectapi_type: ReflectType) -> Self {
        Context {
            reflectapi_type,
            encountered_type_fields: RefCell::new(Vec::new()),
            encountered_type_generics: RefCell::new(Vec::new()),
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    pub fn reflectapi_type(&self) -> ReflectType {
        self.reflectapi_type
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
    pub fn check(self) -> syn::Result<ContextEncounters> {
        let mut errors = self.errors.borrow_mut().take().unwrap().into_iter();

        let mut combined = match errors.next() {
            Some(first) => first,
            None => {
                return Ok(ContextEncounters {
                    fields: self.encountered_type_fields.replace(Vec::new()),
                    generics: self.encountered_type_generics.replace(Vec::new()),
                })
            }
        };

        for rest in errors {
            combined.combine(rest);
        }

        Err(combined)
    }

    /// Add a type reference to actual type definition.
    pub fn encountered_field_type(
        &self,
        type_ref: reflectapi_schema::TypeReference,
        ty: syn::Type,
    ) {
        self.encountered_type_fields
            .borrow_mut()
            .push((type_ref, ty));
    }

    /// Add a type parameter to actual type definition.
    pub fn encountered_generic_type(
        &self,
        type_param: reflectapi_schema::TypeParameter,
        ty: syn::Ident,
    ) {
        self.encountered_type_generics
            .borrow_mut()
            .push((type_param, ty));
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !thread::panicking() && self.errors.borrow().is_some() {
            panic!("forgot to check for errors");
        }
    }
}
