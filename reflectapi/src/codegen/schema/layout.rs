//! The keys a flattened field contributes to the object it's flattened into.
//!
//! Serde writes a flattened field's keys into its parent, and an optional
//! group (a flattened `Option`) writes all of its keys or none of them.
//! Backends that describe the parent's keys, such as an OpenAPI schema or a
//! Python model, read that structure from here.

use std::collections::BTreeSet;

use reflectapi_schema::{Enum, Field, Fields, Representation, Type, TypeReference};

use super::presence::{resolve_field_wire_contract, FieldWireFacts, KeyPresence};

/// Flattening deeper than this is treated as unknown. The outermost flattened
/// field's type is at depth 0, as in Python's `collect_flattened_fields`.
const MAX_DEPTH: usize = 10;

/// Resolves a type name in the typespace the backend is describing.
pub(crate) type Lookup<'a, 'l> = &'l dyn Fn(&str) -> Option<&'a Type>;

/// A `#[serde(flatten)]` field and the keys of its type.
pub(crate) struct FlattenedGroup<'a> {
    pub field: &'a Field,
    /// The group may be absent: its type is an `Option`.
    pub optional: bool,
    /// The layout of the type, or of the `Option`'s argument.
    pub layout: Layout<'a>,
}

pub(crate) enum Layout<'a> {
    Struct(StructLayout<'a>),
    /// The tag, then one layout per variant.
    InternallyTagged {
        enm: &'a Enum,
        tag: &'a str,
        variants: Vec<Layout<'a>>,
    },
    AdjacentlyTagged {
        enm: &'a Enum,
        tag: &'a str,
        content: &'a str,
    },
    /// The keys aren't known from the schema.
    Unknown(Opaque<'a>),
}

/// Why a type's keys aren't known from the schema.
pub(crate) enum Opaque<'a> {
    /// An externally tagged or untagged enum.
    Enum(&'a Enum),
    /// A primitive: a unit type such as `()`, which has no keys, or a type
    /// such as `u32`, which can't be flattened.
    Primitive(&'a str),
    /// The type isn't in the typespace.
    Unresolved(String),
    /// Nested deeper than `MAX_DEPTH` flattens.
    TooDeep,
    /// A tuple struct, a transparent struct without exactly one field, or an
    /// `Option` without an argument.
    Unsupported,
}

/// A struct's fields in declaration order.
pub(crate) struct StructLayout<'a> {
    pub members: Vec<Member<'a>>,
}

pub(crate) enum Member<'a> {
    /// A named field with a key of its own.
    Key(&'a Field),
    Flattened(FlattenedGroup<'a>),
}

impl<'a> FlattenedGroup<'a> {
    /// `field` must be flattened.
    pub fn of(field: &'a Field, lookup: Lookup<'a, '_>) -> Self {
        Self::at_depth(field, lookup, 0)
    }

    fn at_depth(field: &'a Field, lookup: Lookup<'a, '_>, depth: usize) -> Self {
        let contract = resolve_field_wire_contract(FieldWireFacts::of(field));
        let optional = !contract.flattened_group_required();
        let type_ref = match (optional, field.type_ref.arguments.as_slice()) {
            (false, _) => Some(&field.type_ref),
            (true, [inner]) => Some(inner),
            (true, _) => None,
        };
        Self {
            field,
            optional,
            layout: type_ref.map_or(Layout::Unknown(Opaque::Unsupported), |type_ref| {
                Layout::of_type(type_ref, lookup, depth)
            }),
        }
    }
}

impl<'a> Layout<'a> {
    pub fn of_type(type_ref: &TypeReference, lookup: Lookup<'a, '_>, depth: usize) -> Self {
        if depth > MAX_DEPTH {
            return Layout::Unknown(Opaque::TooDeep);
        }
        match lookup(&type_ref.name) {
            Some(Type::Struct(strukt)) if strukt.transparent() => {
                let mut fields = strukt.fields();
                match (fields.next(), fields.next()) {
                    (Some(inner), None) => Layout::of_type(&inner.type_ref, lookup, depth + 1),
                    _ => Layout::Unknown(Opaque::Unsupported),
                }
            }
            Some(Type::Struct(strukt)) => Layout::of_fields(&strukt.fields, lookup, depth),
            Some(Type::Enum(enm)) => match &enm.representation {
                Representation::Internal { tag } => Layout::InternallyTagged {
                    enm,
                    tag,
                    variants: enm
                        .variants()
                        .map(|variant| match &variant.fields {
                            Fields::Unnamed(fields) => match fields.as_slice() {
                                [newtype] => Layout::of_type(&newtype.type_ref, lookup, depth + 1),
                                _ => Layout::Unknown(Opaque::Unsupported),
                            },
                            fields => Layout::of_fields(fields, lookup, depth),
                        })
                        .collect(),
                },
                Representation::Adjacent { tag, content } => {
                    Layout::AdjacentlyTagged { enm, tag, content }
                }
                Representation::External | Representation::None => {
                    Layout::Unknown(Opaque::Enum(enm))
                }
            },
            Some(Type::Primitive(primitive)) => Layout::Unknown(Opaque::Primitive(&primitive.name)),
            None => Layout::Unknown(Opaque::Unresolved(type_ref.name.clone())),
        }
    }

    fn of_fields(fields: &'a Fields, lookup: Lookup<'a, '_>, depth: usize) -> Self {
        let mut members = Vec::new();
        for field in fields.iter() {
            members.push(if field.flattened() {
                Member::Flattened(FlattenedGroup::at_depth(field, lookup, depth + 1))
            } else if field.is_named() {
                Member::Key(field)
            } else {
                return Layout::Unknown(Opaque::Unsupported);
            });
        }
        Layout::Struct(StructLayout { members })
    }

    /// Every key a value may have, or `None` if some aren't known.
    pub fn keys(&self) -> Option<BTreeSet<&'a str>> {
        let mut keys = BTreeSet::new();
        self.collect_keys(&mut keys)?;
        Some(keys)
    }

    fn collect_keys(&self, keys: &mut BTreeSet<&'a str>) -> Option<()> {
        match self {
            Layout::Struct(layout) => {
                for member in &layout.members {
                    match member {
                        Member::Key(field) => {
                            keys.insert(field.serde_name());
                        }
                        Member::Flattened(group) => group.layout.collect_keys(keys)?,
                    }
                }
            }
            Layout::InternallyTagged { tag, variants, .. } => {
                keys.insert(tag);
                for variant in variants {
                    variant.collect_keys(keys)?;
                }
            }
            Layout::AdjacentlyTagged { tag, content, .. } => {
                keys.extend([*tag, *content]);
            }
            Layout::Unknown(_) => return None,
        }
        Some(())
    }

    /// Some key is present in every value, so an absent group isn't a value.
    pub fn has_required_key(&self) -> bool {
        match self {
            Layout::Struct(layout) => layout.members.iter().any(|member| match member {
                Member::Key(field) => is_required(field),
                Member::Flattened(group) => !group.optional && group.layout.has_required_key(),
            }),
            Layout::InternallyTagged { .. } | Layout::AdjacentlyTagged { .. } => true,
            Layout::Unknown(_) => false,
        }
    }
}

impl<'a> StructLayout<'a> {
    /// Every field with a key, through flattened structs, or `None` if a
    /// flattened group isn't a struct.
    pub fn all_fields(&self) -> Option<Vec<&'a Field>> {
        let mut fields = Vec::new();
        for member in &self.members {
            match member {
                Member::Key(field) => fields.push(*field),
                Member::Flattened(FlattenedGroup {
                    layout: Layout::Struct(layout),
                    ..
                }) => fields.extend(layout.all_fields()?),
                Member::Flattened(_) => return None,
            }
        }
        Some(fields)
    }

    /// The fields every value has, through flattened structs that are
    /// always present.
    pub fn required_fields(&self) -> Vec<&'a Field> {
        let mut fields = Vec::new();
        for member in &self.members {
            match member {
                Member::Key(field) if is_required(field) => fields.push(*field),
                Member::Flattened(FlattenedGroup {
                    optional: false,
                    layout: Layout::Struct(layout),
                    ..
                }) => fields.extend(layout.required_fields()),
                _ => {}
            }
        }
        fields
    }

    /// Optional groups of struct type, including those nested in other
    /// flattened structs.
    pub fn optional_struct_groups(&self) -> Vec<&StructLayout<'a>> {
        let mut groups = Vec::new();
        for member in &self.members {
            if let Member::Flattened(FlattenedGroup {
                optional,
                layout: Layout::Struct(layout),
                ..
            }) = member
            {
                if *optional {
                    groups.push(layout);
                }
                groups.extend(layout.optional_struct_groups());
            }
        }
        groups
    }
}

fn is_required(field: &Field) -> bool {
    resolve_field_wire_contract(FieldWireFacts::of(field)).key == KeyPresence::Required
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflectapi_schema::{Enum, Struct, Variant};

    fn field(name: &str, type_name: &str, required: bool) -> Field {
        let mut field = Field::new(name.into(), type_name.into());
        field.required = required;
        field
    }

    fn flattened(name: &str, type_ref: TypeReference) -> Field {
        let mut field = Field::new(name.into(), type_ref);
        field.flattened = true;
        field
    }

    fn option_of(type_name: &str) -> TypeReference {
        TypeReference::new("std::option::Option", vec![type_name.into()])
    }

    fn strukt(name: &str, fields: Vec<Field>) -> Type {
        let mut strukt = Struct::new(name);
        strukt.fields = Fields::Named(fields);
        Type::Struct(strukt)
    }

    fn lookup_in<'t>(types: &'t [Type]) -> impl Fn(&str) -> Option<&'t Type> + 't {
        move |name| types.iter().find(|ty| ty.name() == name)
    }

    #[test]
    fn an_optional_group_lists_its_keys_and_required_fields_in_declaration_order() {
        let types = [
            strukt(
                "Pair",
                vec![
                    field("left", "u32", true),
                    flattened("extra", "Extra".into()),
                    field("right", "u32", false),
                ],
            ),
            strukt("Extra", vec![field("note", "u32", true)]),
        ];
        let parent = flattened("pair", option_of("Pair"));
        let group = FlattenedGroup::of(&parent, &lookup_in(&types));
        assert!(group.optional);
        let Layout::Struct(layout) = &group.layout else {
            panic!("expected a struct layout");
        };
        let names = |fields: Vec<&Field>| fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
        assert_eq!(
            names(layout.all_fields().unwrap()),
            ["left", "note", "right"]
        );
        assert_eq!(names(layout.required_fields()), ["left", "note"]);
        assert_eq!(
            group.layout.keys().unwrap(),
            BTreeSet::from(["left", "note", "right"])
        );
        assert!(group.layout.has_required_key());
    }

    #[test]
    fn a_nested_optional_group_contributes_no_required_key() {
        let types = [
            strukt("Outer", vec![flattened("inner", option_of("Inner"))]),
            strukt("Inner", vec![field("value", "u32", true)]),
        ];
        let layout = Layout::of_type(&"Outer".into(), &lookup_in(&types), 0);
        assert!(!layout.has_required_key());
        assert_eq!(layout.keys().unwrap(), BTreeSet::from(["value"]));
        let Layout::Struct(outer) = &layout else {
            panic!("expected a struct layout");
        };
        assert_eq!(outer.optional_struct_groups().len(), 1);
        assert!(outer.required_fields().is_empty());
    }

    #[test]
    fn an_internally_tagged_enum_has_its_tag_and_every_variant_key() {
        let mut enm = Enum::new("Priority".into());
        enm.representation = Representation::Internal { tag: "kind".into() };
        let mut high = Variant::new("High".into());
        high.fields = Fields::Named(vec![field("deadline", "String", true)]);
        enm.variants = vec![high, Variant::new("Low".into())];
        let types = [Type::Enum(enm)];
        let layout = Layout::of_type(&"Priority".into(), &lookup_in(&types), 0);
        assert_eq!(layout.keys().unwrap(), BTreeSet::from(["deadline", "kind"]));
        assert!(layout.has_required_key());
    }

    #[test]
    fn ten_nested_flattens_are_followed_and_an_eleventh_is_not() {
        let chain = |nested: usize| {
            let mut types = (0..nested)
                .map(|i| {
                    strukt(
                        &format!("S{i}"),
                        vec![flattened("next", format!("S{}", i + 1).as_str().into())],
                    )
                })
                .collect::<Vec<_>>();
            types.push(strukt(
                &format!("S{nested}"),
                vec![field("leaf", "u32", true)],
            ));
            types
        };
        let parent = flattened("group", option_of("S0"));
        let known = |types: &[Type]| {
            FlattenedGroup::of(&parent, &lookup_in(types))
                .layout
                .keys()
                .is_some()
        };
        assert!(known(&chain(10)));
        assert!(!known(&chain(11)));
    }

    #[test]
    fn unknown_keys_make_the_whole_layout_unknown() {
        let mut external = Enum::new("External".into());
        external.representation = Representation::External;
        let types = [
            strukt("Outer", vec![flattened("e", "External".into())]),
            Type::Enum(external),
        ];
        let layout = Layout::of_type(&"Outer".into(), &lookup_in(&types), 0);
        assert!(layout.keys().is_none());
        assert!(matches!(
            Layout::of_type(&"Missing".into(), &lookup_in(&types), 0),
            Layout::Unknown(Opaque::Unresolved(name)) if name == "Missing"
        ));
    }

    #[test]
    fn an_unknown_layout_says_why() {
        let mut external = Enum::new("External".into());
        external.representation = Representation::External;
        let mut tuple = Struct::new("Tuple");
        tuple.fields = Fields::Unnamed(vec![field("0", "u32", true)]);
        let types = [
            Type::Enum(external),
            Type::Struct(tuple),
            Type::Primitive(reflectapi_schema::Primitive::new(
                "u32".into(),
                String::new(),
                Vec::new(),
                None,
            )),
        ];
        let layout = |name: &str| Layout::of_type(&name.into(), &lookup_in(&types), 0);
        assert!(
            matches!(layout("External"), Layout::Unknown(Opaque::Enum(e)) if e.name == "External")
        );
        assert!(matches!(
            layout("Tuple"),
            Layout::Unknown(Opaque::Unsupported)
        ));
        assert!(matches!(
            layout("u32"),
            Layout::Unknown(Opaque::Primitive("u32"))
        ));
        assert!(matches!(
            Layout::of_type(&"u32".into(), &lookup_in(&types), MAX_DEPTH + 1),
            Layout::Unknown(Opaque::TooDeep)
        ));
    }

    #[test]
    fn a_tagged_enum_layout_has_its_definition() {
        let mut internal = Enum::new("Internal".into());
        internal.representation = Representation::Internal { tag: "kind".into() };
        let mut adjacent = Enum::new("Adjacent".into());
        adjacent.representation = Representation::Adjacent {
            tag: "t".into(),
            content: "c".into(),
        };
        let types = [Type::Enum(internal), Type::Enum(adjacent)];
        let layout = |name: &str| Layout::of_type(&name.into(), &lookup_in(&types), 0);
        assert!(
            matches!(layout("Internal"), Layout::InternallyTagged { enm, .. } if enm.name == "Internal")
        );
        assert!(
            matches!(layout("Adjacent"), Layout::AdjacentlyTagged { enm, .. } if enm.name == "Adjacent")
        );
    }

    #[test]
    fn a_transparent_struct_has_the_keys_of_the_type_it_wraps() {
        let mut wrapper = Struct::new("Wrapper");
        wrapper.transparent = true;
        wrapper.fields = Fields::Named(vec![field("inner", "Inner", true)]);
        let types = [
            Type::Struct(wrapper),
            strukt("Inner", vec![field("a", "u32", true)]),
        ];
        let layout = Layout::of_type(&"Wrapper".into(), &lookup_in(&types), 0);
        assert_eq!(layout.keys().unwrap(), BTreeSet::from(["a"]));
    }
}
