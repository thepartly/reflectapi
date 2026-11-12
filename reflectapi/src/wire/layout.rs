//! The keys a flattened field contributes to the object it's flattened into.
//!
//! Serde writes a flattened field's keys into its parent, and an optional
//! group (a flattened `Option`) writes all of its keys or none of them.
//! The builder, for the headers a request-headers type reads, and backends
//! that describe the parent's keys, such as an OpenAPI schema or a Python
//! model, read that structure from here.
//!
//! A generic type's arguments are filled in, so a layout holds the fields of
//! the instantiated type, not those of its definition.

use std::collections::BTreeSet;

use reflectapi_schema::{
    mk_subst, Enum, Field, Fields, Instantiate, Primitive, Representation, Substitute, Type,
    TypeReference,
};

use super::key::{key_presence, FieldWireFacts, KeyPresence};
use super::shape::{wrapped_field, Shape};

/// Flattening deeper than this is treated as unknown. The outermost flattened
/// field's type is at depth 0, as in Python's `collect_flattened_fields`.
pub(crate) const MAX_DEPTH: usize = 10;

/// The unit type, which has no keys.
pub(crate) const UNIT: &str = "std::tuple::Tuple0";

/// Resolves a type name in the typespace the backend is describing.
pub(crate) type Lookup<'a, 'l> = &'l dyn Fn(&str) -> Option<&'a Type>;

/// A `#[serde(flatten)]` field and the keys of its type.
pub(crate) struct FlattenedGroup {
    pub field: Field,
    /// The group may be absent: its type is an `Option`.
    pub optional: bool,
    /// The layout of the type, or of the `Option`'s argument.
    pub layout: Layout,
}

pub(crate) enum Layout {
    Struct(StructLayout),
    /// The tag, then one layout per variant.
    InternallyTagged {
        enm: Box<Enum>,
        tag: String,
        variants: Vec<Layout>,
    },
    AdjacentlyTagged {
        enm: Box<Enum>,
        tag: String,
        content: String,
    },
    /// The keys aren't known from the schema.
    Unknown(Opaque),
}

/// Why a type's keys aren't known from the schema.
pub(crate) enum Opaque {
    /// An externally tagged or untagged enum.
    Enum(Box<Enum>),
    /// A primitive: a unit type such as `()` ([`UNIT`]), which has no keys,
    /// or a type such as `u32`, which can't be flattened.
    Primitive(String),
    /// The type isn't in the typespace.
    Unresolved(String),
    /// Nested deeper than `MAX_DEPTH` flattens.
    TooDeep,
    /// A tuple struct with more than one field, a transparent struct without
    /// exactly one field, an `Option` without an argument, or a generic type
    /// given the wrong number of arguments.
    Unsupported,
}

/// A struct's fields in declaration order.
pub(crate) struct StructLayout {
    pub members: Vec<Member>,
}

pub(crate) enum Member {
    /// A named field with a key of its own.
    Key(Field),
    Flattened(FlattenedGroup),
}

/// A field with a key, found through flattened structs.
pub(crate) struct KeyedField<'s> {
    pub field: &'s Field,
    /// Every value has the key: the field is required, and so is each
    /// flattened group it's found through.
    pub required: bool,
}

impl FlattenedGroup {
    /// `field` must be flattened.
    pub fn of(field: &Field, lookup: Lookup<'_, '_>) -> Self {
        Self::at_depth(field.clone(), lookup, 0)
    }

    fn at_depth(field: Field, lookup: Lookup<'_, '_>, depth: usize) -> Self {
        let optional = key_presence(FieldWireFacts::of(&field))
            != (KeyPresence::Flattened { optional: false });
        let layout = match (optional, field.type_ref.arguments.as_slice()) {
            (false, _) => Layout::of_type(&field.type_ref, lookup, depth),
            (true, [inner]) => Layout::of_type(inner, lookup, depth),
            (true, _) => Layout::Unknown(Opaque::Unsupported),
        };
        Self {
            field,
            optional,
            layout,
        }
    }
}

impl Layout {
    pub fn of_type(type_ref: &TypeReference, lookup: Lookup<'_, '_>, depth: usize) -> Self {
        if depth > MAX_DEPTH {
            return Layout::Unknown(Opaque::TooDeep);
        }
        match lookup(&type_ref.name) {
            Some(Type::Struct(strukt)) => {
                let Some(strukt) = instantiate(strukt, strukt.parameters.len(), type_ref) else {
                    return Layout::Unknown(Opaque::Unsupported);
                };
                match wrapped_field(&strukt) {
                    Some(inner) => Layout::of_type(&inner.type_ref, lookup, depth + 1),
                    None if strukt.transparent() => Layout::Unknown(Opaque::Unsupported),
                    None => Layout::of_fields(strukt.fields, lookup, depth),
                }
            }
            Some(Type::Enum(enm)) => {
                let Some(enm) = instantiate(enm, enm.parameters.len(), type_ref) else {
                    return Layout::Unknown(Opaque::Unsupported);
                };
                let enm = Box::new(enm);
                match &enm.representation {
                    Representation::Internal { tag } => Layout::InternallyTagged {
                        tag: tag.clone(),
                        variants: enm
                            .variants()
                            .map(|variant| match Shape::of(&variant.fields) {
                                Shape::Newtype(newtype) => {
                                    Layout::of_type(&newtype.type_ref, lookup, depth + 1)
                                }
                                Shape::Tuple(_) => Layout::Unknown(Opaque::Unsupported),
                                Shape::Unit | Shape::Struct(_) => {
                                    Layout::of_fields(variant.fields.clone(), lookup, depth)
                                }
                            })
                            .collect(),
                        enm,
                    },
                    Representation::Adjacent { tag, content } => Layout::AdjacentlyTagged {
                        tag: tag.clone(),
                        content: content.clone(),
                        enm,
                    },
                    Representation::External | Representation::None => {
                        Layout::Unknown(Opaque::Enum(enm))
                    }
                }
            }
            Some(Type::Primitive(primitive)) => match pointee(primitive, type_ref) {
                Some(pointee) => Layout::of_type(&pointee, lookup, depth),
                None => Layout::Unknown(Opaque::Primitive(primitive.name.clone())),
            },
            None => Layout::Unknown(Opaque::Unresolved(type_ref.name.clone())),
        }
    }

    fn of_fields(fields: Fields, lookup: Lookup<'_, '_>, depth: usize) -> Self {
        let mut members = Vec::new();
        for field in fields {
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
    pub fn keys(&self) -> Option<BTreeSet<&str>> {
        let mut keys = BTreeSet::new();
        self.collect_keys(&mut keys)?;
        Some(keys)
    }

    fn collect_keys<'s>(&'s self, keys: &mut BTreeSet<&'s str>) -> Option<()> {
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
                keys.extend([tag.as_str(), content.as_str()]);
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

impl StructLayout {
    /// Every field with a key, through flattened structs, or `None` if a
    /// flattened group is of another type than a struct or `()`.
    pub fn all_fields(&self) -> Option<Vec<&Field>> {
        let fields = self.keyed_fields().ok()?;
        Some(fields.into_iter().map(|keyed| keyed.field).collect())
    }

    /// The fields every value has, through flattened structs that are
    /// always present.
    pub fn required_fields(&self) -> Vec<&Field> {
        let mut fields = Vec::new();
        for member in &self.members {
            match member {
                Member::Key(field) if is_required(field) => fields.push(field),
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

    /// Every field with a key, through flattened structs, in declaration
    /// order, each with whether every value has it. A flattened `()` has no
    /// keys. Fails with the first flattened group of another type.
    pub fn keyed_fields(&self) -> Result<Vec<KeyedField<'_>>, &FlattenedGroup> {
        let mut fields = Vec::new();
        self.collect_keyed_fields(true, &mut fields)?;
        Ok(fields)
    }

    fn collect_keyed_fields<'s>(
        &'s self,
        required: bool,
        fields: &mut Vec<KeyedField<'s>>,
    ) -> Result<(), &'s FlattenedGroup> {
        for member in &self.members {
            match member {
                Member::Key(field) => fields.push(KeyedField {
                    field,
                    required: required && is_required(field),
                }),
                Member::Flattened(group) => match &group.layout {
                    Layout::Struct(layout) => {
                        layout.collect_keyed_fields(required && !group.optional, fields)?
                    }
                    Layout::Unknown(Opaque::Primitive(name)) if name == UNIT => {}
                    _ => return Err(group),
                },
            }
        }
        Ok(())
    }

    /// Optional groups of struct type, including those nested in other
    /// flattened structs.
    pub fn optional_struct_groups(&self) -> Vec<&StructLayout> {
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
    key_presence(FieldWireFacts::of(field)) == KeyPresence::Required
}

/// `definition`, which takes `parameters` type arguments, with those of
/// `type_ref` filled in, or `None` if it's given another number.
fn instantiate<T: Clone + Instantiate>(
    definition: &T,
    parameters: usize,
    type_ref: &TypeReference,
) -> Option<T> {
    (parameters == type_ref.arguments.len())
        .then(|| definition.clone().instantiate(&type_ref.arguments))
}

/// The type a pointer such as `Box<T>` or `Arc<T>` holds, which serde
/// writes in its place, flattened or not: the primitive falls back to one of
/// its type parameters. A primitive with a representation of its own, such
/// as `HashSet<T>` or `chrono::DateTime`, isn't a pointer.
pub(crate) fn pointee(primitive: &Primitive, type_ref: &TypeReference) -> Option<TypeReference> {
    let fallback = primitive.fallback()?;
    if primitive.parameters.len() != type_ref.arguments.len()
        || !primitive
            .parameters()
            .any(|parameter| parameter.name() == fallback.name)
    {
        return None;
    }
    let subst = mk_subst(&primitive.parameters, &type_ref.arguments);
    Some(fallback.clone().subst(&subst))
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

    fn generic(name: &str, parameter: &str, fields: Vec<Field>) -> Type {
        let mut strukt = Struct::new(name);
        strukt.parameters = vec![parameter.into()];
        strukt.fields = Fields::Named(fields);
        Type::Struct(strukt)
    }

    fn keyed(layout: &Layout) -> Vec<(String, bool)> {
        let Layout::Struct(layout) = layout else {
            panic!("expected a struct layout");
        };
        let Ok(fields) = layout.keyed_fields() else {
            panic!("expected every key to be known");
        };
        fields
            .into_iter()
            .map(|keyed| (keyed.field.serde_name().to_owned(), keyed.required))
            .collect()
    }

    #[test]
    fn a_generic_type_has_the_keys_of_its_arguments() {
        let types = [
            generic(
                "WithCommon",
                "T",
                vec![
                    field("request_id", "std::string::String", false),
                    flattened("route", "T".into()),
                ],
            ),
            generic("Wrap", "U", vec![flattened("inner", "U".into())]),
            strukt("Route", vec![field("authorization", "u32", true)]),
        ];
        let route = TypeReference::new("Wrap", vec!["Route".into()]);
        let type_ref = TypeReference::new("WithCommon", vec![route]);
        let layout = Layout::of_type(&type_ref, &lookup_in(&types), 0);
        assert_eq!(
            keyed(&layout),
            [
                ("request_id".to_owned(), false),
                ("authorization".to_owned(), true)
            ]
        );
        assert!(matches!(
            Layout::of_type(&"WithCommon".into(), &lookup_in(&types), 0),
            Layout::Unknown(Opaque::Unsupported)
        ));
    }

    #[test]
    fn a_pointer_has_the_keys_of_the_type_it_holds() {
        let types = [
            Type::Primitive(reflectapi_schema::Primitive::new(
                "std::boxed::Box".into(),
                String::new(),
                vec!["T".into()],
                Some("T".into()),
            )),
            strukt(
                "Outer",
                vec![flattened(
                    "boxed",
                    TypeReference::new("std::boxed::Box", vec!["Inner".into()]),
                )],
            ),
            strukt("Inner", vec![field("a", "u32", true)]),
        ];
        let layout = Layout::of_type(&"Outer".into(), &lookup_in(&types), 0);
        assert_eq!(keyed(&layout), [("a".to_owned(), true)]);
    }

    #[test]
    fn keyed_fields_are_required_only_through_groups_that_always_appear() {
        let types = [
            strukt(
                "Outer",
                vec![
                    field("own", "u32", true),
                    flattened("always", "Always".into()),
                    flattened("maybe", option_of("Maybe")),
                    flattened("unit", UNIT.into()),
                ],
            ),
            strukt("Always", vec![field("a", "u32", true)]),
            strukt(
                "Maybe",
                vec![field("b", "u32", true), field("c", "u32", false)],
            ),
            Type::Primitive(reflectapi_schema::Primitive::new(
                UNIT.into(),
                String::new(),
                Vec::new(),
                None,
            )),
        ];
        let layout = Layout::of_type(&"Outer".into(), &lookup_in(&types), 0);
        let expected = [("own", true), ("a", true), ("b", false), ("c", false)]
            .map(|(name, required)| (name.to_owned(), required));
        assert_eq!(keyed(&layout), expected);
    }

    #[test]
    fn keyed_fields_fail_on_a_group_that_is_not_a_struct() {
        let mut external = Enum::new("External".into());
        external.representation = Representation::External;
        let types = [
            strukt("Outer", vec![flattened("e", "External".into())]),
            Type::Enum(external),
        ];
        let Layout::Struct(layout) = Layout::of_type(&"Outer".into(), &lookup_in(&types), 0) else {
            panic!("expected a struct layout");
        };
        assert!(matches!(layout.keyed_fields(), Err(group) if group.field.name == "e"));
        assert!(layout.all_fields().is_none());
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
        tuple.fields = Fields::Unnamed(vec![field("0", "u32", true), field("1", "u32", true)]);
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
            Layout::Unknown(Opaque::Primitive(name)) if name == "u32"
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

    #[test]
    fn a_newtype_struct_has_the_keys_of_the_type_it_wraps() {
        let mut wrapper = Struct::new("Wrapper");
        wrapper.fields = Fields::Unnamed(vec![field("0", "Inner", true)]);
        let types = [
            Type::Struct(wrapper),
            strukt("Inner", vec![field("a", "u32", true)]),
        ];
        let layout = Layout::of_type(&"Wrapper".into(), &lookup_in(&types), 0);
        assert_eq!(layout.keys().unwrap(), BTreeSet::from(["a"]));
    }
}
