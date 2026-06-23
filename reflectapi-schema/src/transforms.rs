use crate::{Field, TypeReference, Typespace};

/// Unwraps transparent wrappers (e.g. `Arc<T>` → `T`) on the field's type reference.
pub fn fallback_recursively(field: &mut Field, schema: &Typespace) {
    field.type_ref.fallback_recursively(schema);
}

/// Makes a field required in the schema.
///
/// - `std::option::Option<T>` → required `T`
/// - `reflectapi::Option<T>` → required `std::option::Option<T>`
///   (removes the "undefined" state, keeps nullable)
/// - Other types → marks the field as required without changing the type
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

/// Makes a field non-nullable in the schema.
///
/// - `std::option::Option<T>` → `T` (required unchanged)
/// - `reflectapi::Option<T>` → `T` (required unchanged)
///   (removes the "null" state, keeps optionality)
/// - Other types → no-op (already non-nullable)
pub fn make_nonnullable(field: &mut Field, _schema: &Typespace) {
    if field.type_ref.name() == "std::option::Option"
        || field.type_ref.name() == "reflectapi::Option"
    {
        if let Some(inner) = field.type_ref.arguments().next().cloned() {
            field.type_ref = inner;
        }
    }
}

/// Makes a field both required and non-nullable in the schema.
///
/// - `std::option::Option<T>` → required `T`
/// - `reflectapi::Option<T>` → required `T`
///   (removes both "undefined" and "null" states)
/// - Other types → marks the field as required without changing the type
pub fn make_required_and_nonnullable(field: &mut Field, schema: &Typespace) {
    make_nonnullable(field, schema);
    make_required(field, schema);
}
