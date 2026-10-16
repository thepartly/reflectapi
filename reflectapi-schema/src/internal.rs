use super::*;

fn replace_type_references_for_type_ref(
    this: &mut TypeReference,
    resolved_type_ref: &TypeReference,
    declaring_type_parameters: &Vec<TypeParameter>,
) {
    if declaring_type_parameters.is_empty() {
        // This code needs to replace unresolved type reference to resolved type reference
        // For example, 'Vec<u8>' without parameters to std::vec::Vec with parameters [u8].
        this.name.clone_from(&resolved_type_ref.name);
        this.arguments.clone_from(&resolved_type_ref.arguments);
    } else {
        let unresolved_parsed = this.clone();
        this.name.clone_from(&resolved_type_ref.name);
        this.arguments.clone_from(&resolved_type_ref.arguments);

        replace_specific_type_ref_by_generic_for_type_ref(
            this,
            &unresolved_parsed,
            declaring_type_parameters,
        )
    }
}

fn replace_specific_type_ref_by_generic_for_type_ref(
    this: &mut TypeReference,
    unresolved_with_generics: &TypeReference,
    declaring_type_parameters: &Vec<TypeParameter>,
) {
    if declaring_type_parameters
        .iter()
        .any(|i| i.name() == unresolved_with_generics.name())
    {
        *this = unresolved_with_generics.clone();
    }

    for t in this
        .arguments
        .iter_mut()
        .zip(unresolved_with_generics.arguments.iter())
    {
        replace_specific_type_ref_by_generic_for_type_ref(t.0, t.1, declaring_type_parameters);
    }
}

pub(crate) fn replace_type_references_for_type(
    this: &mut Type,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    input: bool,
) {
    match this {
        Type::Primitive(_) => {}
        Type::Struct(s) => replace_type_references_for_struct(s, remap, schema, input),
        Type::Enum(e) => replace_type_references_for_enum(e, remap, schema, input),
    }
}

fn replace_type_references_for_struct(
    this: &mut Struct,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    input: bool,
) {
    let input = input && matches!(this.fields, Fields::Named(_));
    for field in this.fields.iter_mut() {
        replace_type_references_for_field(field, remap, schema, &this.parameters, input);
    }
}

fn replace_type_references_for_field(
    this: &mut Field,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    declaring_type_parameters: &Vec<TypeParameter>,
    input: bool,
) {
    if let Some(new_type_ref) = remap.get(&this.type_ref) {
        replace_type_references_for_type_ref(
            &mut this.type_ref,
            new_type_ref,
            declaring_type_parameters,
        );
    }
    // Serde's missing-field deserializer accepts a plain option as None.
    // Resolve this only for derived inputs, after aliases are canonicalized
    // and generic parameters restored. User transforms run afterwards and
    // remain authoritative over the reflected wire contract.
    if input && !this.deserialize_with && accepts_missing_option(&this.type_ref, schema) {
        this.required = false;
    }
    if let Some(transform_callback_fn) = this.transform_callback_fn {
        transform_callback_fn(this, schema);
    }
}

// These standard Serde pointer implementations delegate deserialization to T.
// Arbitrary primitive fallbacks describe a representation, not a decoder's
// missing-field behavior, so they must not imply option-style leniency.
fn accepts_missing_option(type_ref: &TypeReference, schema: &Typespace) -> bool {
    let mut current = type_ref.clone();
    let mut visited = std::collections::HashSet::new();
    while visited.insert(current.clone()) {
        match current.name() {
            "std::option::Option" | "reflectapi::Option" => return true,
            "std::boxed::Box" | "std::rc::Rc" | "std::sync::Arc" => {
                let Some(inner) = current.fallback_once(schema) else {
                    return false;
                };
                current = inner;
            }
            _ => return false,
        }
    }
    false
}

fn replace_type_references_for_enum(
    this: &mut Enum,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    input: bool,
) {
    for variant in this.variants.iter_mut() {
        replace_type_references_for_variant(variant, remap, schema, &this.parameters, input);
    }
}

fn replace_type_references_for_variant(
    this: &mut Variant,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    declaring_type_parameters: &Vec<TypeParameter>,
    input: bool,
) {
    let input = input && matches!(this.fields, Fields::Named(_));
    for field in this.fields.iter_mut() {
        replace_type_references_for_field(field, remap, schema, declaring_type_parameters, input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_specific_with_generic() {
        let mut resolved = TypeReference::new(
            "Vec2",
            vec![
                TypeReference::new("Vec", vec!["u8".into()]),
                TypeReference::new("Vec", vec!["u16".into(), "u8".into()]),
            ],
        );
        let unresolved = TypeReference::new(
            "Vec",
            vec![
                TypeReference::new("Vec", vec!["T".into()]),
                TypeReference::new("Vec", vec!["U".into(), "T".into()]),
            ],
        );
        let declaring_type_parameters = vec![TypeParameter::from("T"), TypeParameter::from("U")];
        replace_specific_type_ref_by_generic_for_type_ref(
            &mut resolved,
            &unresolved,
            &declaring_type_parameters,
        );
        assert_eq!(
            resolved,
            TypeReference::new(
                "Vec2",
                vec![
                    TypeReference::new("Vec", vec!["T".into()]),
                    TypeReference::new("Vec", vec!["U".into(), "T".into()])
                ]
            )
        );
    }

    #[test]
    fn test_replace_specific_with_generic_more() {
        let mut resolved = TypeReference::new(
            "Vec2",
            vec![
                TypeReference::new("Vec", vec!["u8".into()]),
                TypeReference::new("Vec", vec!["u16".into(), "u8".into()]),
            ],
        );
        let unresolved = TypeReference::new(
            "Vec",
            vec![
                TypeReference::new("T", vec![]),
                TypeReference::new(
                    "Vec",
                    vec![
                        TypeReference::new("U", vec![]),
                        TypeReference::new("X", vec![]),
                    ],
                ),
            ],
        );
        let declaring_type_parameters = vec![TypeParameter::from("T"), TypeParameter::from("U")];
        replace_specific_type_ref_by_generic_for_type_ref(
            &mut resolved,
            &unresolved,
            &declaring_type_parameters,
        );
        assert_eq!(
            resolved,
            TypeReference::new(
                "Vec2",
                vec![
                    TypeReference::new("T", vec![]),
                    TypeReference::new("Vec", vec!["U".into(), "u8".into()])
                ]
            )
        );
    }

    #[test]
    fn test_replace_circular_with_generic() {
        let mut resolved = TypeReference::new("GenericStruct", vec!["A".into()]);
        let unresolved = TypeReference::new(
            "GenericStruct",
            vec![TypeReference::new("GenericStruct", vec!["u8".into()])],
        );
        let declaring_type_parameters = vec![TypeParameter::from("A")];
        replace_specific_type_ref_by_generic_for_type_ref(
            &mut resolved,
            &unresolved,
            &declaring_type_parameters,
        );
        assert_eq!(
            resolved,
            TypeReference::new("GenericStruct", vec!["A".into()])
        );
    }
}
