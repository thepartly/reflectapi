use crate::{Field, TypeReference, Typespace};

/// Unwraps transparent wrappers (e.g. `Arc<T>` → `T`) on the field's type reference.
pub fn fallback_recursively(field: &mut Field, schema: &Typespace) {
    field.type_ref.fallback_recursively(schema);
}

/// Makes an optional field required in the schema.
///
/// - `std::option::Option<T>` → required `T`
/// - `reflectapi::Option<T>` → required `std::option::Option<T>`
///   (removes the "undefined" state, keeps nullable)
pub fn make_required(field: &mut Field, _schema: &Typespace) {
    if field.type_ref.name() == "std::option::Option" {
        if let Some(inner) = field.type_ref.arguments().next().cloned() {
            field.type_ref = inner;
        }
    } else if field.type_ref.name() == "reflectapi::Option" {
        let arguments = field.type_ref.arguments().cloned().collect::<Vec<_>>();
        field.type_ref = TypeReference::new("std::option::Option", arguments);
    }
    field.required = true;
}
