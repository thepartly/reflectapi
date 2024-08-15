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
) {
    match this {
        Type::Primitive(_) => {}
        Type::Struct(s) => replace_type_references_for_struct(s, remap, schema),
        Type::Enum(e) => replace_type_references_for_enum(e, remap, schema),
    }
}

fn replace_type_references_for_struct(
    this: &mut Struct,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
) {
    for field in this.fields.iter_mut() {
        replace_type_references_for_field(field, remap, schema, &this.parameters);
    }
}

fn replace_type_references_for_field(
    this: &mut Field,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    declaring_type_parameters: &Vec<TypeParameter>,
) {
    if let Some(new_type_ref) = remap.get(&this.type_ref) {
        replace_type_references_for_type_ref(
            &mut this.type_ref,
            new_type_ref,
            declaring_type_parameters,
        );
    }
    if let Some(transform_callback_fn) = this.transform_callback_fn {
        transform_callback_fn(&mut this.type_ref, schema);
    }
}

fn replace_type_references_for_enum(
    this: &mut Enum,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
) {
    for variant in this.variants.iter_mut() {
        replace_type_references_for_variant(variant, remap, schema, &this.parameters);
    }
}

fn replace_type_references_for_variant(
    this: &mut Variant,
    remap: &std::collections::HashMap<TypeReference, TypeReference>,
    schema: &Typespace,
    declaring_type_parameters: &Vec<TypeParameter>,
) {
    for field in this.fields.iter_mut() {
        replace_type_references_for_field(field, remap, schema, declaring_type_parameters);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_specific_with_generic() {
        let mut resolved = TypeReference::new(
            "Vec2".into(),
            vec![
                TypeReference::new("Vec".into(), vec!["u8".into()]),
                TypeReference::new("Vec".into(), vec!["u16".into(), "u8".into()]),
            ],
        );
        let unresolved = TypeReference::new(
            "Vec".into(),
            vec![
                TypeReference::new("Vec".into(), vec!["T".into()]),
                TypeReference::new("Vec".into(), vec!["U".into(), "T".into()]),
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
                "Vec2".into(),
                vec![
                    TypeReference::new("Vec".into(), vec!["T".into()]),
                    TypeReference::new("Vec".into(), vec!["U".into(), "T".into()])
                ]
            )
        );
    }

    #[test]
    fn test_replace_specific_with_generic_more() {
        let mut resolved = TypeReference::new(
            "Vec2".into(),
            vec![
                TypeReference::new("Vec".into(), vec!["u8".into()]),
                TypeReference::new("Vec".into(), vec!["u16".into(), "u8".into()]),
            ],
        );
        let unresolved = TypeReference::new(
            "Vec".into(),
            vec![
                TypeReference::new("T".into(), vec![]),
                TypeReference::new(
                    "Vec".into(),
                    vec![
                        TypeReference::new("U".into(), vec![]),
                        TypeReference::new("X".into(), vec![]),
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
                "Vec2".into(),
                vec![
                    TypeReference::new("T".into(), vec![]),
                    TypeReference::new("Vec".into(), vec!["U".into(), "u8".into()])
                ]
            )
        );
    }

    #[test]
    fn test_replace_circular_with_generic() {
        let mut resolved = TypeReference::new("GenericStruct".into(), vec!["A".into()]);
        let unresolved = TypeReference::new(
            "GenericStruct".into(),
            vec![TypeReference::new(
                "GenericStruct".into(),
                vec!["u8".into()],
            )],
        );
        let declaring_type_parameters = vec![TypeParameter::from("A")];
        replace_specific_type_ref_by_generic_for_type_ref(
            &mut resolved,
            &unresolved,
            &declaring_type_parameters,
        );
        assert_eq!(
            resolved,
            TypeReference::new("GenericStruct".into(), vec!["A".into()])
        );
    }
}
