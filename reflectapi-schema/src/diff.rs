//! Finds the changes between two schemas that break existing clients.
//!
//! Schemas are compared function by function, and each function in both
//! directions: its input, what clients send, and its output, what they
//! receive. A request breaks when the server
//! stops accepting something an old client sends; a response breaks when an
//! old client can no longer read what the server sends. So the same change can
//! be compatible one way and breaking the other.
//!
//! Types are compared by the JSON they read and write, not by name: generic
//! arguments are substituted, newtypes and transparent wrappers unwrapped and
//! flattened fields inlined, so renaming or restructuring a type without
//! changing its JSON is compatible.
//!
//! This checks wire compatibility only. Renaming a type is reported as
//! compatible, yet it can still break code that names the type in a
//! generated client.
//!
//! Removing something clients rely on is judged by [`Options`]: whether it was
//! deprecated, whether its sunset has come, how long it has been deprecated,
//! and whether an earlier schema already had it deprecated.
//!
//! [`Kind`] and [`Change::key`] are stable: acknowledgements name changes by
//! them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::str::FromStr;

use crate::deprecation::is_iso_date;
use crate::{
    Enum, Field, Fields, Function, Instantiate, OutputType, Representation, Schema,
    SerializationMode, Struct, Type, TypeReference, Typespace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Severity {
    Breaking,
    /// Breaking, but of a kind listed in [`Options::warn`].
    Warning,
    Compatible,
}

/// What clients send, or what they receive. Private, because public variants
/// named `Input` and `Output` would make rustc spell out the `reflectapi::Input`
/// and `reflectapi::Output` traits in every error that names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Direction {
    Input,
    Output,
}

impl Direction {
    /// Requests break when what the server accepts narrows, responses when
    /// what clients may receive widens.
    fn breaks(self, effect: Effect) -> bool {
        matches!(
            (self, effect),
            (Direction::Input, Effect::Narrows) | (Direction::Output, Effect::Widens)
        )
    }

    fn severity(self, effect: Effect) -> Severity {
        if self.breaks(effect) {
            Severity::Breaking
        } else {
            Severity::Compatible
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Direction::Input => "input",
            Direction::Output => "output",
        }
    }
}

/// How a change moves the set of values a type allows.
#[derive(Debug, Clone, Copy)]
enum Effect {
    Narrows,
    Widens,
}

macro_rules! kinds {
    ($($kind:ident => $name:literal,)*) => {
        /// What changed, stable across releases so acknowledgements can name it.
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        #[non_exhaustive]
        pub enum Kind {
            $(#[serde(rename = $name)] $kind,)*
        }

        impl Kind {
            pub const ALL: &'static [Kind] = &[$(Kind::$kind,)*];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Kind::$kind => $name,)*
                }
            }
        }
    };
}

kinds! {
    FunctionAdded => "function-added",
    FunctionRemoved => "function-removed",
    PathChanged => "path-changed",
    ReadonlyRemoved => "readonly-removed",
    SerializationRemoved => "serialization-removed",
    OutputKindChanged => "output-kind-changed",
    BodyAdded => "body-added",
    BodyRemoved => "body-removed",
    FieldAdded => "field-added",
    RequiredFieldAdded => "required-field-added",
    FieldRemoved => "field-removed",
    RequiredFieldRemoved => "required-field-removed",
    FieldBecameRequired => "field-became-required",
    FieldBecameOptional => "field-became-optional",
    BecameNullable => "became-nullable",
    BecameNotNullable => "became-not-nullable",
    TypeChanged => "type-changed",
    TypeRenamed => "type-renamed",
    NumberChanged => "number-changed",
    FormatChanged => "format-changed",
    RepresentationChanged => "representation-changed",
    DiscriminantChanged => "discriminant-changed",
    VariantAdded => "variant-added",
    VariantRemoved => "variant-removed",
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Kind::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| format!("unknown change kind `{s}`"))
    }
}

/// A calendar date, written `YYYY-MM-DD`, which orders as its text does.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(String);

impl FromStr for Date {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_iso_date(s) {
            Ok(Date(s.to_owned()))
        } else {
            Err(format!(
                "invalid date `{s}`, expected a calendar date as YYYY-MM-DD"
            ))
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[non_exhaustive]
pub struct Change {
    pub severity: Severity,
    pub function: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    pub kind: Kind,
    pub message: String,
    /// Names this change in an acknowledgement:
    /// `<function> <location> [<kind>]`, such as
    /// `pets.list output body.items[].age [required-field-removed]`.
    pub key: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub was_deprecated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunset: Option<String>,
    /// The reason given by the acknowledgement that accepts this change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged: Option<String>,
}

impl Change {
    fn at(
        function: &str,
        location: String,
        kind: Kind,
        severity: Severity,
        message: String,
    ) -> Self {
        let key = if location.is_empty() {
            format!("{function} [{kind}]")
        } else {
            format!("{function} {location} [{kind}]")
        };
        Change {
            severity,
            function: function.to_owned(),
            location,
            kind,
            message,
            key,
            was_deprecated: false,
            deprecated_since: None,
            sunset: None,
            acknowledged: None,
        }
    }

    fn deprecated(self, deprecated: Deprecated) -> Self {
        Change {
            was_deprecated: deprecated.was_deprecated,
            deprecated_since: deprecated.since,
            sunset: deprecated.sunset,
            ..self
        }
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function)?;
        if !self.location.is_empty() {
            write!(f, " {}", self.location)?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Build one from [`Options::default`] and set the fields you need.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Options {
    /// The date removals are judged on. When set, removing a function, a
    /// required response field or a request variant is compatible once the
    /// item was deprecated and its sunset is on or before this date. When not
    /// set, every such removal is breaking.
    pub today: Option<Date>,
    /// With `today`, a removal is compatible only if the item's
    /// `deprecated_since` is on or before this date, such as a week before
    /// `today` to require a week's notice.
    pub notice_cutoff: Option<Date>,
    /// When not empty, only functions with at least one of these tags are compared.
    pub include_tags: BTreeSet<String>,
    /// Functions with any of these tags are not compared.
    pub exclude_tags: BTreeSet<String>,
    /// Breaking changes of these kinds are reported as [`Severity::Warning`].
    pub warn: BTreeSet<Kind>,
    /// The schema as it was a notice period ago, such as the default branch a
    /// week back. With `today`, a removal is compatible only if the item was
    /// already deprecated in it, which proves how long the deprecation stood
    /// where `deprecated_since`, written by the same author as the removal,
    /// cannot.
    pub deprecated_in: Option<Schema>,
}

impl Options {
    fn compares(&self, function: &Function) -> bool {
        (self.include_tags.is_empty()
            || function.tags.iter().any(|t| self.include_tags.contains(t)))
            && function.tags.iter().all(|t| !self.exclude_tags.contains(t))
    }
}

/// Compares the wire contract of two schemas, function by function, with
/// removals judged and changes chosen by [`Options`].
pub fn diff(old: &Schema, new: &Schema, options: &Options) -> Vec<Change> {
    let snapshot = options.deprecated_in.as_ref().map(deprecated_items);
    let mut differ = Differ::new(old, new, options, snapshot);
    differ.functions();
    let mut changes = differ.changes;
    for change in &mut changes {
        if change.severity == Severity::Breaking && options.warn.contains(&change.kind) {
            change.severity = Severity::Warning;
        }
    }
    changes.sort();
    changes.dedup();
    changes
}

/// Where a change is: a function, a direction, and the path to the value.
#[derive(Clone)]
struct Site {
    function: String,
    direction: Direction,
    path: Vec<Segment>,
}

#[derive(Clone)]
enum Segment {
    /// `body`, `headers`, `error` or `stream item`.
    Part(&'static str),
    Field(WireKey),
    Variant(String),
    /// The item of a one-argument container, such as a `Vec`.
    Item,
    /// An argument of a container with several, such as a map's key.
    Argument(usize),
    /// An element of a tuple.
    Position(usize),
}

impl Site {
    fn child(&self, segment: Segment) -> Site {
        let mut child = self.clone();
        child.path.push(segment);
        child
    }

    fn location(&self) -> String {
        let mut out = self.direction.as_str().to_owned();
        for segment in &self.path {
            match segment {
                Segment::Part(part) => out += &format!(" {part}"),
                Segment::Field(key) => out += &format!(".{key}"),
                Segment::Variant(name) => out += &format!("::{name}"),
                Segment::Item => out += "[]",
                Segment::Argument(i) => out += &format!("<{i}>"),
                Segment::Position(i) => out += &format!(".{i}"),
            }
        }
        out
    }
}

#[derive(Clone)]
struct Deprecated {
    was_deprecated: bool,
    since: Option<String>,
    sunset: Option<String>,
}

impl Deprecated {
    fn of(deprecated: bool, since: Option<&str>, sunset: Option<&str>) -> Self {
        Deprecated {
            was_deprecated: deprecated,
            since: since.map(str::to_owned),
            sunset: sunset.map(str::to_owned),
        }
    }

    fn describe(&self, what: &str) -> String {
        match (self.was_deprecated, &self.sunset) {
            (true, Some(sunset)) => format!("{what} (deprecated, sunset {sunset})"),
            (true, None) => format!("{what} (deprecated)"),
            (false, _) => what.to_owned(),
        }
    }
}

enum Shape {
    Nullable(TypeReference),
    Primitive {
        name: String,
        arguments: Vec<TypeReference>,
        format: Option<String>,
    },
    Struct(Struct),
    Enum(Enum),
    Unknown(TypeReference),
}

impl Shape {
    fn kind(&self) -> String {
        match self {
            Shape::Nullable(_) => "nullable".into(),
            Shape::Primitive { name, .. } => format!("`{name}`"),
            Shape::Struct(s) => format!("struct `{}`", s.name),
            Shape::Enum(e) => format!("enum `{}`", e.name),
            Shape::Unknown(r) => format!("`{}`", r.name),
        }
    }
}

const NULLABLE: [&str; 2] = ["std::option::Option", "reflectapi::Option"];

fn resolve(types: &Typespace, r: &TypeReference) -> Shape {
    if NULLABLE.contains(&r.name()) && r.arguments.len() == 1 {
        return Shape::Nullable(r.arguments[0].clone());
    }
    match types.get_type(r.name()) {
        Some(Type::Primitive(p)) => Shape::Primitive {
            name: p.name.clone(),
            arguments: r.arguments.clone(),
            format: p.format.clone(),
        },
        Some(Type::Struct(s)) => {
            let s = s.clone().instantiate(&r.arguments);
            // serde writes a newtype, transparent or not, as the value it wraps.
            if s.is_alias() {
                if let Some(field) = s.fields().next() {
                    return resolve(types, field.type_ref());
                }
            }
            Shape::Struct(s)
        }
        Some(Type::Enum(e)) => Shape::Enum(e.clone().instantiate(&r.arguments)),
        None => Shape::Unknown(r.clone()),
    }
}

/// A field's place in the JSON: its name, or, for a flattened value that
/// can't be inlined, the Rust field holding it, which serde never writes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WireKey {
    Flattened(String),
    Field(String),
}

impl fmt::Display for WireKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireKey::Flattened(name) => write!(f, "(flattened {name})"),
            WireKey::Field(name) => f.write_str(name),
        }
    }
}

struct WireField {
    /// The struct, or `<enum>::<variant>`, that declares the field, and its
    /// name there.
    declared_in: String,
    serde_name: String,
    type_ref: TypeReference,
    required: bool,
    deprecated: Deprecated,
}

type WireFields = BTreeMap<WireKey, WireField>;

/// The fields a struct has on the wire, with flattened structs inlined.
fn wire_fields(
    types: &Typespace,
    direction: Direction,
    owner: &str,
    fields: &[Field],
) -> WireFields {
    let mut out = BTreeMap::new();
    collect_wire_fields(types, direction, owner, fields, true, &mut out);
    out
}

fn collect_wire_fields(
    types: &Typespace,
    direction: Direction,
    owner: &str,
    fields: &[Field],
    required: bool,
    out: &mut WireFields,
) {
    for field in fields {
        // serde reads a missing `Option` field as `None` unless the field
        // has its own `deserialize_with`.
        let missing_reads_as_none = direction == Direction::Input
            && field.type_ref().name() == "std::option::Option"
            && !field.deserialize_with;
        let mut required = required && field.required() && !missing_reads_as_none;
        if field.flattened() {
            let inner = match resolve(types, field.type_ref()) {
                Shape::Nullable(inner) => {
                    required = false;
                    inner
                }
                _ => field.type_ref().clone(),
            };
            if let Shape::Struct(Struct {
                name,
                fields: Fields::Named(fields),
                ..
            }) = resolve(types, &inner)
            {
                collect_wire_fields(types, direction, &name, &fields, required, out);
                continue;
            }
            // A flattened map collects the keys nothing else reads, so it is
            // empty, not missing, when there are none.
            if is_map(types, &inner) {
                required = false;
            }
        }
        let key = if field.flattened() {
            WireKey::Flattened(field.name().to_owned())
        } else {
            WireKey::Field(field.serde_name().to_owned())
        };
        out.insert(
            key,
            WireField {
                declared_in: owner.to_owned(),
                serde_name: field.serde_name().to_owned(),
                type_ref: field.type_ref().clone(),
                required,
                deprecated: Deprecated::of(
                    field.deprecated(),
                    field.deprecated_since.as_deref(),
                    field.sunset.as_deref(),
                ),
            },
        );
    }
}

const STRING: &str = "std::string::String";

/// How many fallbacks or newtypes deep a type is followed.
const LONGEST_CHAIN: usize = 16;

/// The type serde writes a value of `r` as: a newtype's, transparent or not,
/// is the value it wraps.
fn unwrap_newtypes(types: &Typespace, r: &TypeReference) -> TypeReference {
    let mut current = r.clone();
    for _ in 0..LONGEST_CHAIN {
        let Some(Type::Struct(s)) = types.get_type(current.name()) else {
            break;
        };
        if !s.is_alias() {
            break;
        }
        let s = s.clone().instantiate(&current.arguments);
        let Some(field) = s.fields().next() else {
            break;
        };
        current = field.type_ref().clone();
    }
    current
}

/// A type, then each type it falls back to in turn.
fn fallbacks<'a>(
    types: &'a Typespace,
    r: &TypeReference,
) -> impl Iterator<Item = TypeReference> + 'a {
    std::iter::successors(Some(r.clone()), |current| current.fallback_once(types))
        .take(LONGEST_CHAIN)
}

/// Whether a type is written as a JSON string: `String`, a primitive reflectapi
/// writes as one, or one that falls back to `String`.
fn is_string(types: &Typespace, r: &TypeReference) -> bool {
    const STRINGS: [&str; 3] = [STRING, "char", "uuid::Uuid"];
    fallbacks(types, r).any(|t| STRINGS.contains(&t.name()))
}

fn is_map(types: &Typespace, r: &TypeReference) -> bool {
    const MAPS: [&str; 3] = [
        "std::collections::HashMap",
        "std::collections::BTreeMap",
        "indexmap::IndexMap",
    ];
    fallbacks(types, r).any(|t| MAPS.contains(&t.name()))
}

/// Pairs the flattened values neither side could inline: first those of the
/// same type, then the one left on each side, if one is.
fn flattened_renames(old: &WireFields, new: &WireFields) -> BTreeMap<WireKey, WireKey> {
    let unmatched = |a: &WireFields, b: &WireFields| {
        a.keys()
            .filter(|k| matches!(k, WireKey::Flattened(_)) && !b.contains_key(*k))
            .cloned()
            .collect::<Vec<_>>()
    };
    let (mut olds, mut news) = (unmatched(old, new), unmatched(new, old));
    let mut renames = BTreeMap::new();
    olds.retain(
        |o| match news.iter().position(|n| new[n].type_ref == old[o].type_ref) {
            Some(i) => {
                renames.insert(o.clone(), news.remove(i));
                false
            }
            None => true,
        },
    );
    if let ([o], [n]) = (olds.as_slice(), news.as_slice()) {
        renames.insert(o.clone(), n.clone());
    }
    renames
}

fn numeric_fits(from: &str, to: &str) -> Option<bool> {
    fn rank(name: &str) -> Option<(char, u32)> {
        let kind = match name {
            "i8" | "i16" | "i32" | "i64" | "i128" => 'i',
            "u8" | "u16" | "u32" | "u64" | "u128" => 'u',
            "f32" | "f64" => 'f',
            _ => return None,
        };
        Some((kind, name[1..].parse().ok()?))
    }
    let (from_kind, from_bits) = rank(from)?;
    let (to_kind, to_bits) = rank(to)?;
    Some(match (from_kind, to_kind) {
        (a, b) if a == b => from_bits <= to_bits,
        ('u', 'i') => from_bits < to_bits,
        ('u' | 'i', 'f') => (to_bits == 64 && from_bits <= 32) || from_bits <= 16,
        _ => false,
    })
}

/// What an earlier schema must show deprecated for a removal to be allowed.
/// Fields and variants are named by the type that declares them, where
/// deprecation is declared, so a route added since still finds them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Proof {
    Function(String),
    Field {
        direction: Direction,
        owner: String,
        field: String,
    },
    Variant {
        direction: Direction,
        owner: String,
        variant: String,
    },
}

fn deprecated_items(schema: &Schema) -> BTreeSet<Proof> {
    let mut out: BTreeSet<Proof> = schema
        .functions
        .iter()
        .filter(|f| f.deprecated())
        .map(|f| Proof::Function(f.name.clone()))
        .collect();
    for (direction, types) in [
        (Direction::Input, &schema.input_types),
        (Direction::Output, &schema.output_types),
    ] {
        let field = |owner: &str, field: &Field| Proof::Field {
            direction,
            owner: owner.to_owned(),
            field: field.serde_name().to_owned(),
        };
        for ty in types.types() {
            match ty {
                Type::Struct(s) => out.extend(
                    s.fields()
                        .filter(|f| f.deprecated())
                        .map(|f| field(&s.name, f)),
                ),
                Type::Enum(e) => {
                    for v in e.variants() {
                        if v.deprecated() {
                            out.insert(Proof::Variant {
                                direction,
                                owner: e.name.clone(),
                                variant: v.serde_name().to_owned(),
                            });
                        }
                        let owner = format!("{}::{}", e.name, v.serde_name());
                        out.extend(
                            v.fields()
                                .filter(|f| f.deprecated())
                                .map(|f| field(&owner, f)),
                        );
                    }
                }
                Type::Primitive(_) => {}
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy)]
enum Side {
    Old,
    New,
}

struct Differ<'a> {
    old: &'a Schema,
    new: &'a Schema,
    options: &'a Options,
    changes: Vec<Change>,
    /// The pairs of types being compared, to stop at recursion.
    comparing: HashSet<(Direction, TypeReference, TypeReference)>,
    /// The items deprecated in [`Options::deprecated_in`].
    snapshot: Option<BTreeSet<Proof>>,
}

impl<'a> Differ<'a> {
    fn new(
        old: &'a Schema,
        new: &'a Schema,
        options: &'a Options,
        snapshot: Option<BTreeSet<Proof>>,
    ) -> Self {
        Differ {
            old,
            new,
            options,
            changes: Vec::new(),
            comparing: HashSet::new(),
            snapshot,
        }
    }

    fn types(&self, side: Side, direction: Direction) -> &'a Typespace {
        let schema = match side {
            Side::Old => self.old,
            Side::New => self.new,
        };
        match direction {
            Direction::Input => &schema.input_types,
            Direction::Output => &schema.output_types,
        }
    }

    fn report(&mut self, site: &Site, kind: Kind, severity: Severity, message: impl Into<String>) {
        self.changes.push(Change::at(
            &site.function,
            site.location(),
            kind,
            severity,
            message.into(),
        ));
    }

    /// Judges removing something clients rely on. It is compatible once it
    /// was deprecated and its sunset has come, and as [`Options`] asks, it has
    /// been deprecated long enough and was already deprecated in the earlier
    /// schema.
    fn judge_removal(
        &self,
        what: &str,
        proof: &Proof,
        deprecated: &Deprecated,
    ) -> (Severity, String) {
        let breaking = |message: String| (Severity::Breaking, message);
        let Some(today) = &self.options.today else {
            return breaking(deprecated.describe(what));
        };
        if !deprecated.was_deprecated {
            return breaking(format!("{what} without deprecation"));
        }
        let sunset = match deprecated.sunset.as_deref().map(str::parse::<Date>) {
            None => return breaking(format!("{what}, deprecated without a sunset")),
            Some(Err(_)) => {
                return breaking(format!(
                    "{what}, deprecated with an invalid sunset `{}`",
                    deprecated.sunset.as_deref().unwrap_or_default()
                ))
            }
            Some(Ok(sunset)) => sunset,
        };
        if &sunset > today {
            return breaking(format!("{what} before its sunset {sunset}"));
        }
        if let Some(cutoff) = &self.options.notice_cutoff {
            let since = match deprecated.since.as_deref().map(str::parse::<Date>) {
                None => {
                    return breaking(format!(
                        "{what}, deprecated without a date to count its notice from"
                    ))
                }
                Some(Err(_)) => {
                    return breaking(format!(
                        "{what}, deprecated on an invalid date `{}`",
                        deprecated.since.as_deref().unwrap_or_default()
                    ))
                }
                Some(Ok(since)) => since,
            };
            if &since > cutoff {
                return breaking(format!(
                    "{what}, deprecated on {since}, later than the notice cutoff {cutoff}"
                ));
            }
        }
        if self.snapshot.as_ref().is_some_and(|s| !s.contains(proof)) {
            return breaking(format!("{what}, not deprecated in the earlier schema"));
        }
        (
            Severity::Compatible,
            format!("{what} after its sunset {sunset}"),
        )
    }

    fn functions(&mut self) {
        let compared = |schema: &'a Schema| -> BTreeMap<&'a str, &'a Function> {
            schema
                .functions
                .iter()
                .filter(|f| self.options.compares(f))
                .map(|f| (f.name(), f))
                .collect()
        };
        let old = compared(self.old);
        let new = compared(self.new);

        for (name, old_fn) in &old {
            match new.get(name) {
                Some(new_fn) => self.function(old_fn, new_fn),
                None => {
                    let deprecated = Deprecated::of(
                        old_fn.deprecated(),
                        old_fn.deprecated_since.as_deref(),
                        old_fn.sunset.as_deref(),
                    );
                    let proof = Proof::Function(name.to_string());
                    let (severity, message) =
                        self.judge_removal("function removed", &proof, &deprecated);
                    self.changes.push(
                        Change::at(
                            name,
                            String::new(),
                            Kind::FunctionRemoved,
                            severity,
                            message,
                        )
                        .deprecated(deprecated),
                    );
                }
            }
        }
        for name in new.keys().filter(|name| !old.contains_key(*name)) {
            self.changes.push(Change::at(
                name,
                String::new(),
                Kind::FunctionAdded,
                Severity::Compatible,
                "function added".into(),
            ));
        }
    }

    fn function(&mut self, old: &Function, new: &Function) {
        let site = |direction, path: Vec<Segment>| Site {
            function: old.name.clone(),
            direction,
            path,
        };
        let part = |direction, part| site(direction, vec![Segment::Part(part)]);
        let on_function = |differ: &mut Self, kind, message: String| {
            differ.changes.push(Change::at(
                &old.name,
                String::new(),
                kind,
                Severity::Breaking,
                message,
            ))
        };

        if old.path != new.path {
            on_function(
                self,
                Kind::PathChanged,
                format!("path changed from `{}` to `{}`", old.path, new.path),
            );
        }
        if old.readonly && !new.readonly {
            on_function(
                self,
                Kind::ReadonlyRemoved,
                "no longer readonly, so no longer accepts GET".into(),
            );
        }
        let modes = |f: &Function| -> BTreeSet<SerializationMode> {
            let modes: BTreeSet<SerializationMode> = f.serialization().cloned().collect();
            if modes.is_empty() {
                BTreeSet::from([SerializationMode::Json])
            } else {
                modes
            }
        };
        for mode in modes(old).difference(&modes(new)) {
            on_function(
                self,
                Kind::SerializationRemoved,
                format!("no longer supports {mode:?} serialization"),
            );
        }

        self.optional(
            &part(Direction::Input, "body"),
            old.input_type(),
            new.input_type(),
        );
        self.optional(
            &part(Direction::Input, "headers"),
            old.input_headers(),
            new.input_headers(),
        );
        match (old.output_type(), new.output_type()) {
            (OutputType::Complete { output_type: o }, OutputType::Complete { output_type: n }) => {
                self.optional(&part(Direction::Output, "body"), o.as_ref(), n.as_ref())
            }
            (OutputType::Stream { item_type: o }, OutputType::Stream { item_type: n }) => {
                self.compare(&part(Direction::Output, "stream item"), o, n)
            }
            (OutputType::Complete { .. }, OutputType::Stream { .. }) => self.report(
                &site(Direction::Output, vec![]),
                Kind::OutputKindChanged,
                Severity::Breaking,
                "changed from a single response to a stream",
            ),
            (OutputType::Stream { .. }, OutputType::Complete { .. }) => self.report(
                &site(Direction::Output, vec![]),
                Kind::OutputKindChanged,
                Severity::Breaking,
                "changed from a stream to a single response",
            ),
        }
        self.optional(
            &part(Direction::Output, "error"),
            old.error_type.as_ref(),
            new.error_type.as_ref(),
        );
        self.optional(
            &part(Direction::Output, "headers"),
            old.response_headers(),
            new.response_headers(),
        );
    }

    fn optional(&mut self, site: &Site, old: Option<&TypeReference>, new: Option<&TypeReference>) {
        match (old, new) {
            (None, None) => {}
            (Some(old), Some(new)) => self.compare(site, old, new),
            (None, Some(new)) => match site.direction {
                Direction::Input => {
                    let types = self.types(Side::New, site.direction);
                    let required: Vec<String> = match resolve(types, new) {
                        Shape::Struct(Struct {
                            name,
                            fields: Fields::Named(fields),
                            ..
                        }) => wire_fields(types, site.direction, &name, &fields)
                            .into_iter()
                            .filter(|(_, f)| f.required)
                            .map(|(key, _)| key.to_string())
                            .collect(),
                        Shape::Nullable(_) => vec![],
                        _ => vec!["(value)".into()],
                    };
                    if required.is_empty() {
                        self.report(
                            site,
                            Kind::BodyAdded,
                            Severity::Compatible,
                            "now read, all of it optional",
                        );
                    } else {
                        self.report(
                            site,
                            Kind::BodyAdded,
                            Severity::Breaking,
                            format!("now required: {}", required.join(", ")),
                        );
                    }
                }
                Direction::Output => {
                    self.report(site, Kind::BodyAdded, Severity::Compatible, "now returned")
                }
            },
            (Some(_), None) => self.report(
                site,
                Kind::BodyRemoved,
                site.direction.severity(Effect::Widens),
                "no longer exchanged",
            ),
        }
    }

    fn compare(&mut self, site: &Site, old: &TypeReference, new: &TypeReference) {
        let pair = (site.direction, old.clone(), new.clone());
        if self.comparing.insert(pair.clone()) {
            self.compare_types(site, old, new);
            self.comparing.remove(&pair);
        }
    }

    fn compare_types(&mut self, site: &Site, old: &TypeReference, new: &TypeReference) {
        let old = &unwrap_newtypes(self.types(Side::Old, site.direction), old);
        let new = &unwrap_newtypes(self.types(Side::New, site.direction), new);
        if old.name != new.name {
            // Not an `Effect`: a change such as `i32` to `u32` both widens and narrows.
            if let Some(widens) = numeric_fits(&old.name, &new.name) {
                let narrows = numeric_fits(&new.name, &old.name).unwrap_or(false);
                let compatible = match site.direction {
                    Direction::Input => widens,
                    Direction::Output => narrows,
                };
                self.report(
                    site,
                    Kind::NumberChanged,
                    if compatible {
                        Severity::Compatible
                    } else {
                        Severity::Breaking
                    },
                    format!("changed from `{}` to `{}`", old.name, new.name),
                );
                return;
            }
        }

        let old_shape = resolve(self.types(Side::Old, site.direction), old);
        let new_shape = resolve(self.types(Side::New, site.direction), new);

        match (old_shape, new_shape) {
            (Shape::Nullable(o), Shape::Nullable(n)) => self.compare(site, &o, &n),
            (Shape::Nullable(o), _) => {
                self.report(
                    site,
                    Kind::BecameNotNullable,
                    site.direction.severity(Effect::Narrows),
                    "no longer nullable",
                );
                self.compare(site, &o, new);
            }
            (_, Shape::Nullable(n)) => {
                self.report(
                    site,
                    Kind::BecameNullable,
                    site.direction.severity(Effect::Widens),
                    "now nullable",
                );
                self.compare(site, old, &n);
            }
            (
                Shape::Primitive {
                    name: on,
                    arguments: oa,
                    format: of,
                },
                Shape::Primitive {
                    name: nn,
                    arguments: na,
                    format: nf,
                },
            ) if on == nn => {
                // A format newly declared on the same type describes what it
                // always accepted, as when reflectapi starts declaring one.
                if let (Some(of), Some(nf)) = (&of, &nf) {
                    if of != nf {
                        self.report(
                            site,
                            Kind::FormatChanged,
                            Severity::Breaking,
                            format!("format changed from `{of}` to `{nf}`"),
                        );
                    }
                }
                self.arguments(site, &oa, &na);
            }
            (Shape::Primitive { .. }, Shape::Primitive { .. }) => self.fallback(site, old, new),
            (Shape::Struct(o), Shape::Struct(n)) => {
                self.renamed(site, &o.name, &n.name);
                self.fields(
                    site,
                    (&o.name, &o.fields),
                    (&n.name, &n.fields),
                    n.deny_unknown_fields,
                );
            }
            (Shape::Enum(o), Shape::Enum(n)) => {
                self.renamed(site, &o.name, &n.name);
                self.enums(site, &o, &n);
            }
            (Shape::Unknown(o), Shape::Unknown(n)) if o.name == n.name => {
                self.arguments(site, &o.arguments, &n.arguments)
            }
            (Shape::Primitive { .. }, _) | (_, Shape::Primitive { .. }) => {
                self.fallback(site, old, new)
            }
            (o, n) => {
                let core = |differ: &Self, side, shape: &Shape, other: &str| match shape {
                    Shape::Struct(s) => differ.flattened_core(side, site.direction, s, other),
                    _ => None,
                };
                match (
                    core(self, Side::Old, &o, &new.name),
                    core(self, Side::New, &n, &old.name),
                ) {
                    (None, Some((core, extras))) => {
                        self.compare(site, old, &core);
                        self.wire_field_changes(site, &BTreeMap::new(), &extras, false);
                    }
                    (Some((core, extras)), None) => {
                        self.compare(site, &core, new);
                        self.wire_field_changes(site, &extras, &BTreeMap::new(), false);
                    }
                    _ => self.report(
                        site,
                        Kind::TypeChanged,
                        Severity::Breaking,
                        format!("changed from {} to {}", o.kind(), n.kind()),
                    ),
                }
            }
        }
    }

    /// A struct holding a flattened value that can't be inlined, such as
    /// `struct Wrapper { #[serde(flatten)] data: SomeEnum, extra: Option<X> }`,
    /// reads and writes that value with its other fields beside it. With
    /// several such values, the one returned is the one of type `other`, the
    /// type on the other side of the diff.
    fn flattened_core(
        &self,
        side: Side,
        direction: Direction,
        s: &Struct,
        other: &str,
    ) -> Option<(TypeReference, WireFields)> {
        let Fields::Named(fields) = &s.fields else {
            return None;
        };
        let types = self.types(side, direction);
        let mut wire = wire_fields(types, direction, &s.name, fields);
        // A flattened map only collects the keys nothing else reads.
        let flattened: Vec<&WireKey> = wire
            .iter()
            .filter(|(k, f)| matches!(k, WireKey::Flattened(_)) && !is_map(types, &f.type_ref))
            .map(|(k, _)| k)
            .collect();
        let key = match flattened.as_slice() {
            [only] => (*only).clone(),
            several => {
                let mut same = several.iter().filter(|k| wire[**k].type_ref.name == other);
                let key = (*same.next()?).clone();
                if same.next().is_some() {
                    return None;
                }
                key
            }
        };
        let core = wire.remove(&key)?;
        Some((core.type_ref, wire))
    }

    fn arguments(&mut self, site: &Site, old: &[TypeReference], new: &[TypeReference]) {
        if old.len() != new.len() {
            self.report(
                site,
                Kind::TypeChanged,
                Severity::Breaking,
                format!("now takes {} type arguments, not {}", new.len(), old.len()),
            );
            return;
        }
        for (i, (o, n)) in old.iter().zip(new).enumerate() {
            let segment = if old.len() == 1 {
                Segment::Item
            } else {
                Segment::Argument(i)
            };
            self.compare(&site.child(segment), o, n);
        }
    }

    /// Primitives such as `uuid::Uuid` or `Box<T>` declare the type they fall
    /// back to; when one side falls back to the other, the wire form is the
    /// same apart from the string format the narrower one promises.
    fn fallback(&mut self, site: &Site, old: &TypeReference, new: &TypeReference) {
        let old_types = self.types(Side::Old, site.direction);
        let new_types = self.types(Side::New, site.direction);

        let chain = |types: &Typespace, from: &TypeReference, to: &str| {
            fallbacks(types, from).find(|t| t.name == to)
        };
        let format = |types: &Typespace, r: &TypeReference| match types.get_type(r.name()) {
            Some(Type::Primitive(p)) => p.format.clone(),
            _ => None,
        };

        if let Some(widened) = chain(old_types, old, &new.name) {
            match format(old_types, old) {
                Some(format) => self.format_changed(site, old, new, &format, Effect::Widens),
                None => self.renamed(site, &old.name, &new.name),
            }
            self.compare(site, &widened, new);
        } else if let Some(narrowed) = chain(new_types, new, &old.name) {
            match format(new_types, new) {
                Some(format) => self.format_changed(site, old, new, &format, Effect::Narrows),
                None => self.renamed(site, &old.name, &new.name),
            }
            self.compare(site, old, &narrowed);
        } else if new.name == STRING && is_string(old_types, old) {
            self.string_changed(site, old, new, format(old_types, old), Effect::Widens);
        } else if old.name == STRING && is_string(new_types, new) {
            self.string_changed(site, old, new, format(new_types, new), Effect::Narrows);
        } else {
            self.report(
                site,
                Kind::TypeChanged,
                Severity::Breaking,
                format!("changed from `{}` to `{}`", old.name, new.name),
            );
        }
    }

    /// A string type such as `uuid::Uuid` or `char` replaced by `String`,
    /// which widens what the value may be, or the reverse, which narrows it.
    fn string_changed(
        &mut self,
        site: &Site,
        old: &TypeReference,
        new: &TypeReference,
        format: Option<String>,
        effect: Effect,
    ) {
        match format {
            Some(format) => self.format_changed(site, old, new, &format, effect),
            None => self.report(
                site,
                Kind::TypeChanged,
                site.direction.severity(effect),
                format!("changed from `{}` to `{}`", old.name, new.name),
            ),
        }
    }

    fn format_changed(
        &mut self,
        site: &Site,
        old: &TypeReference,
        new: &TypeReference,
        format: &str,
        effect: Effect,
    ) {
        let only = match effect {
            Effect::Widens => "no longer only",
            Effect::Narrows => "now only",
        };
        self.report(
            site,
            Kind::FormatChanged,
            site.direction.severity(effect),
            format!(
                "changed from `{}` to `{}`, {only} {format}",
                old.name, new.name
            ),
        );
    }

    fn renamed(&mut self, site: &Site, old: &str, new: &str) {
        if old != new {
            self.report(
                site,
                Kind::TypeRenamed,
                Severity::Compatible,
                format!("type `{old}` is now `{new}`"),
            );
        }
    }

    /// Compares the fields of two structs or variants, each given with the
    /// name of the type that declares them, and whether the new one rejects
    /// fields it doesn't know.
    fn fields(
        &mut self,
        site: &Site,
        old: (&str, &Fields),
        new: (&str, &Fields),
        rejects_unknown: bool,
    ) {
        let ((old_owner, old), (new_owner, new)) = (old, new);
        let kind = |fields: &Fields| match fields {
            Fields::Named(_) => "named fields",
            Fields::Unnamed(_) => "a tuple",
            Fields::None => "no fields",
        };
        match (old, new) {
            (Fields::None, Fields::None) => {}
            (Fields::Unnamed(o), Fields::Unnamed(n)) => {
                if o.len() != n.len() {
                    self.report(
                        site,
                        Kind::TypeChanged,
                        Severity::Breaking,
                        format!("tuple of {} is now a tuple of {}", o.len(), n.len()),
                    );
                    return;
                }
                let newtype = o.len() == 1;
                for (i, (o, n)) in o.iter().zip(n).enumerate() {
                    let child = if newtype {
                        site.clone()
                    } else {
                        site.child(Segment::Position(i))
                    };
                    self.compare(&child, o.type_ref(), n.type_ref());
                }
            }
            (Fields::Named(o), Fields::Named(n)) => {
                let old = wire_fields(
                    self.types(Side::Old, site.direction),
                    site.direction,
                    old_owner,
                    o,
                );
                let new = wire_fields(
                    self.types(Side::New, site.direction),
                    site.direction,
                    new_owner,
                    n,
                );
                self.wire_field_changes(site, &old, &new, rejects_unknown);
            }
            (o, n) => self.report(
                site,
                Kind::TypeChanged,
                Severity::Breaking,
                format!("changed from {} to {}", kind(o), kind(n)),
            ),
        }
    }

    fn wire_field_changes(
        &mut self,
        site: &Site,
        old: &WireFields,
        new: &WireFields,
        rejects_unknown: bool,
    ) {
        let renamed = flattened_renames(old, new);
        for (key, o) in old {
            let child = site.child(Segment::Field(key.clone()));
            let Some(n) = new
                .get(key)
                .or_else(|| renamed.get(key).and_then(|k| new.get(k)))
            else {
                let (kind, what) = match (site.direction, o.required) {
                    (Direction::Input, _) if rejects_unknown => (
                        Kind::FieldRemoved,
                        "field removed, the server rejects it if sent",
                    ),
                    (Direction::Input, _) => (
                        Kind::FieldRemoved,
                        "field removed, the server ignores it if sent",
                    ),
                    (Direction::Output, true) => {
                        (Kind::RequiredFieldRemoved, "required field removed")
                    }
                    (Direction::Output, false) => (Kind::FieldRemoved, "optional field removed"),
                };
                let breaks = match site.direction {
                    Direction::Input => rejects_unknown,
                    Direction::Output => o.required,
                };
                let (severity, message) = if breaks {
                    let proof = Proof::Field {
                        direction: site.direction,
                        owner: o.declared_in.clone(),
                        field: o.serde_name.clone(),
                    };
                    self.judge_removal(what, &proof, &o.deprecated)
                } else {
                    (Severity::Compatible, o.deprecated.describe(what))
                };
                self.changes.push(
                    Change::at(&site.function, child.location(), kind, severity, message)
                        .deprecated(o.deprecated.clone()),
                );
                continue;
            };
            match (o.required, n.required) {
                (false, true) => self.report(
                    &child,
                    Kind::FieldBecameRequired,
                    site.direction.severity(Effect::Narrows),
                    "now required",
                ),
                (true, false) => self.report(
                    &child,
                    Kind::FieldBecameOptional,
                    site.direction.severity(Effect::Widens),
                    "now optional",
                ),
                _ => {}
            }
            self.compare(&child, &o.type_ref, &n.type_ref);
        }

        for (key, n) in new
            .iter()
            .filter(|(key, _)| !old.contains_key(*key) && !renamed.values().any(|r| r == *key))
        {
            let child = site.child(Segment::Field(key.clone()));
            let (kind, severity, message) = match (site.direction, n.required) {
                (Direction::Input, true) => (
                    Kind::RequiredFieldAdded,
                    Severity::Breaking,
                    "required field added",
                ),
                (Direction::Input, false) => (
                    Kind::FieldAdded,
                    Severity::Compatible,
                    "optional field added",
                ),
                (Direction::Output, _) => (Kind::FieldAdded, Severity::Compatible, "field added"),
            };
            self.report(&child, kind, severity, message);
        }
    }

    fn enums(&mut self, site: &Site, old: &Enum, new: &Enum) {
        if old.representation != new.representation {
            let describe = |r: &Representation| match r {
                Representation::External => "externally tagged".to_string(),
                Representation::Internal { tag } => format!("internally tagged by `{tag}`"),
                Representation::Adjacent { tag, content } => {
                    format!("adjacently tagged by `{tag}` and `{content}`")
                }
                Representation::None => "untagged".to_string(),
            };
            self.report(
                site,
                Kind::RepresentationChanged,
                Severity::Breaking,
                format!(
                    "changed from {} to {}",
                    describe(&old.representation),
                    describe(&new.representation)
                ),
            );
            return;
        }

        let old_variants: BTreeMap<&str, _> = old.variants().map(|v| (v.serde_name(), v)).collect();
        let new_variants: BTreeMap<&str, _> = new.variants().map(|v| (v.serde_name(), v)).collect();

        for (name, o) in &old_variants {
            let child = site.child(Segment::Variant(name.to_string()));
            let Some(n) = new_variants.get(name) else {
                let deprecated = Deprecated::of(
                    o.deprecated(),
                    o.deprecated_since.as_deref(),
                    o.sunset.as_deref(),
                );
                let (severity, message) = if site.direction.breaks(Effect::Narrows) {
                    let proof = Proof::Variant {
                        direction: site.direction,
                        owner: old.name.clone(),
                        variant: name.to_string(),
                    };
                    self.judge_removal("variant removed", &proof, &deprecated)
                } else {
                    (Severity::Compatible, deprecated.describe("variant removed"))
                };
                self.changes.push(
                    Change::at(
                        &site.function,
                        child.location(),
                        Kind::VariantRemoved,
                        severity,
                        message,
                    )
                    .deprecated(deprecated),
                );
                continue;
            };
            if o.untagged != n.untagged {
                let message = if n.untagged {
                    "now untagged, written as its content alone"
                } else {
                    "no longer untagged, written with its tag"
                };
                self.report(
                    &child,
                    Kind::RepresentationChanged,
                    Severity::Breaking,
                    message,
                );
            }
            if o.discriminant != n.discriminant {
                self.report(
                    &child,
                    Kind::DiscriminantChanged,
                    Severity::Breaking,
                    "discriminant changed",
                );
            }
            // An internally tagged unit variant is an object holding only its tag,
            // so giving it fields adds them to that object.
            let internal = matches!(old.representation, Representation::Internal { .. });
            let no_fields = Fields::Named(Vec::new());
            let unit = |fields: &Fields| internal && matches!(fields, Fields::None);
            let old_fields = if unit(&o.fields) {
                &no_fields
            } else {
                &o.fields
            };
            let new_fields = if unit(&n.fields) {
                &no_fields
            } else {
                &n.fields
            };
            self.fields(
                &child,
                (&format!("{}::{name}", old.name), old_fields),
                (&format!("{}::{name}", new.name), new_fields),
                new.deny_unknown_fields,
            );
        }

        // Old clients read a new variant as unknown when the enum is open or
        // has a catch-all variant.
        let tolerant = old.open || old.variants().any(|v| v.other);
        for name in new_variants
            .keys()
            .filter(|n| !old_variants.contains_key(*n))
        {
            let child = site.child(Segment::Variant(name.to_string()));
            let (severity, message) = match (site.direction.breaks(Effect::Widens), tolerant) {
                (false, _) => (Severity::Compatible, "variant added"),
                (true, true) => (
                    Severity::Compatible,
                    "variant added, old clients read it as unknown",
                ),
                (true, false) => (
                    Severity::Breaking,
                    "variant added, old clients can't read it",
                ),
            };
            self.report(&child, Kind::VariantAdded, severity, message);
        }
    }
}

/// A breaking change someone has accepted, read from a line
/// `- <key>: <reason>`, where the key is [`Change::key`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Acknowledgement {
    pub key: String,
    pub reason: String,
    /// Where it was read, such as `acks/2026-11-20.md:3`.
    pub source: String,
}

/// An acknowledgement line that can't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InvalidAcknowledgement {
    pub source: String,
    pub line: String,
    pub problem: String,
}

impl fmt::Display for InvalidAcknowledgement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.source, self.problem, self.line)
    }
}

impl std::error::Error for InvalidAcknowledgement {}

/// Reads acknowledgements from a file's text. Each line starting with `- `
/// must be `- <key>: <reason>`; other lines are free text. A reason may not
/// be empty or a `<...>` placeholder.
pub fn parse_acknowledgements(
    text: &str,
    origin: &str,
) -> Result<Vec<Acknowledgement>, InvalidAcknowledgement> {
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let source = format!("{origin}:{}", number + 1);
        let Some(entry) = line.trim().strip_prefix("- ").map(str::trim_start) else {
            continue;
        };
        let invalid = |problem: &str| InvalidAcknowledgement {
            source: source.clone(),
            line: line.to_owned(),
            problem: problem.to_owned(),
        };
        let key_end = entry
            .match_indices(']')
            .map(|(i, _)| i)
            .find(|&end| {
                let key = &entry[..=end];
                key.rfind(" [")
                    .is_some_and(|start| key[start + 2..end].parse::<Kind>().is_ok())
            })
            .ok_or_else(|| invalid("expected `- <function> <location> [<kind>]: <reason>`"))?;
        let reason = entry[key_end + 1..]
            .trim_start()
            .strip_prefix(':')
            .map(str::trim)
            .unwrap_or_default();
        if reason.is_empty() || (reason.starts_with('<') && reason.ends_with('>')) {
            return Err(invalid("give the reason this break is safe"));
        }
        out.push(Acknowledgement {
            key: entry[..=key_end].to_owned(),
            reason: reason.to_owned(),
            source,
        });
    }
    Ok(out)
}

/// Marks each breaking change that an acknowledgement names as
/// [`Change::acknowledged`], and returns the acknowledgements that named none.
pub fn acknowledge(
    changes: &mut [Change],
    acknowledgements: &[Acknowledgement],
) -> Vec<Acknowledgement> {
    let mut used = vec![false; acknowledgements.len()];
    for change in changes
        .iter_mut()
        .filter(|c| c.severity == Severity::Breaking)
    {
        if let Some(i) = acknowledgements.iter().position(|a| a.key == change.key) {
            change.acknowledged = Some(acknowledgements[i].reason.clone());
            used[i] = true;
        }
    }
    acknowledgements
        .iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|(a, _)| a.clone())
        .collect()
}
