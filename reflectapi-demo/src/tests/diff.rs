//! `reflectapi::diff` on schemas the derives build, so each case is checked
//! against what serde and reflectapi actually emit for the Rust types.
//!
//! Each case defines the old and new types in their own modules and diffs
//! the schemas of a route that both reads and returns them. Renames that come
//! only from those modules are left out of the snapshots.

#![allow(dead_code, deprecated)]

use reflectapi::diff::{diff, Kind, Options};

use super::into_schema;

trait Contract:
    reflectapi::Input
    + reflectapi::Output
    + serde::Serialize
    + serde::de::DeserializeOwned
    + Send
    + 'static
{
}

impl<T> Contract for T where
    T: reflectapi::Input
        + reflectapi::Output
        + serde::Serialize
        + serde::de::DeserializeOwned
        + Send
        + 'static
{
}

fn changes<Old: Contract, New: Contract>() -> String {
    changes_with::<Old, New>(&Options::default())
}

fn changes_with<Old: Contract, New: Contract>(options: &Options) -> String {
    diff(&into_schema::<Old>(), &into_schema::<New>(), options)
        .iter()
        .filter(|c| c.kind != Kind::TypeRenamed)
        .map(|c| format!("{:?} {c}\n", c.severity))
        .collect()
}

macro_rules! contract {
    ($($item:item)*) => {
        $(
            #[derive(
                reflectapi::Input,
                reflectapi::Output,
                serde::Serialize,
                serde::Deserialize,
            )]
            $item
        )*
    };
}

mod fields_added {
    pub mod old {
        contract! {
            pub struct Pet {
                pub name: String,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub name: String,
                pub nickname: Option<String>,
                #[serde(default)]
                pub age: u32,
                pub kind: String,
            }
        }
    }
}

#[test]
fn a_request_may_leave_out_an_option_or_a_defaulted_field() {
    insta::assert_snapshot!(changes::<fields_added::old::Pet, fields_added::new::Pet>(), @"
    Breaking inout_test input body.kind: required field added
    Compatible inout_test input body.age: optional field added
    Compatible inout_test input body.nickname: optional field added
    Compatible inout_test output body.age: field added
    Compatible inout_test output body.kind: field added
    Compatible inout_test output body.nickname: field added
    ");
}

mod fields_removed {
    pub mod old {
        contract! {
            pub struct Pet {
                pub name: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub nickname: Option<String>,
                pub age: u32,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub name: String,
            }
        }
    }
}

#[test]
fn a_response_may_drop_only_what_it_could_already_leave_out() {
    insta::assert_snapshot!(changes::<fields_removed::old::Pet, fields_removed::new::Pet>(), @"
    Breaking inout_test output body.age: required field removed
    Compatible inout_test input body.age: field removed, the server ignores it if sent
    Compatible inout_test input body.nickname: field removed, the server ignores it if sent
    Compatible inout_test output body.nickname: optional field removed
    ");
}

mod strict_fields_removed {
    pub mod old {
        contract! {
            #[serde(deny_unknown_fields)]
            pub struct Pet {
                pub name: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub nickname: Option<String>,
            }
        }
    }
    pub mod new {
        contract! {
            #[serde(deny_unknown_fields)]
            pub struct Pet {
                pub name: String,
            }
        }
    }
}

#[test]
fn a_request_that_rejects_unknown_fields_may_not_drop_one() {
    insta::assert_snapshot!(changes::<strict_fields_removed::old::Pet, strict_fields_removed::new::Pet>(), @"
    Breaking inout_test input body.nickname: field removed, the server rejects it if sent
    Compatible inout_test output body.nickname: optional field removed
    ");
}

mod nullable {
    pub mod old {
        contract! {
            pub struct Pet {
                pub age: u32,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub age: Option<u32>,
            }
        }
    }
}

#[test]
fn a_field_that_may_be_null_breaks_responses() {
    insta::assert_snapshot!(changes::<nullable::old::Pet, nullable::new::Pet>(), @"
    Breaking inout_test output body.age: now nullable
    Compatible inout_test input body.age: now optional
    Compatible inout_test input body.age: now nullable
    ");
}

mod flattened_struct {
    pub mod old {
        contract! {
            pub struct Pet {
                pub name: String,
                pub age: u32,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Identity {
                pub name: String,
            }
            pub struct Details {
                pub age: u32,
            }
            pub struct Pet {
                #[serde(flatten)]
                pub identity: Identity,
                #[serde(flatten)]
                pub details: Option<Details>,
            }
        }
    }
}

#[test]
fn flattened_structs_are_read_as_their_fields() {
    insta::assert_snapshot!(changes::<flattened_struct::old::Pet, flattened_struct::new::Pet>(), @"
    Breaking inout_test output body.age: now optional
    Compatible inout_test input body.age: now optional
    ");
}

mod flattened_enum {
    pub mod old {
        contract! {
            #[serde(tag = "kind")]
            pub enum Command {
                Feed { grams: u32 },
                Walk,
            }
        }
    }
    pub mod new {
        contract! {
            #[serde(tag = "kind")]
            pub enum Command {
                Feed { grams: u32 },
                Walk,
            }
            pub struct Envelope {
                #[serde(flatten)]
                pub action: Command,
                #[serde(default)]
                pub dry_run: bool,
                #[serde(flatten)]
                pub extra: std::collections::HashMap<String, String>,
            }
        }
    }
}

#[test]
fn an_enum_wrapped_with_optional_fields_reads_what_it_read() {
    insta::assert_snapshot!(changes::<flattened_enum::old::Command, flattened_enum::new::Envelope>(), @"
    Compatible inout_test input body.(flattened extra): optional field added
    Compatible inout_test input body.dry_run: optional field added
    Compatible inout_test output body.(flattened extra): field added
    Compatible inout_test output body.dry_run: field added
    ");
}

mod unit_variant {
    pub mod old {
        contract! {
            #[serde(tag = "kind")]
            pub enum Event {
                Opened { at: u32 },
                Closed,
            }
        }
    }
    pub mod new {
        contract! {
            #[serde(tag = "kind")]
            pub enum Event {
                Opened { at: u32 },
                Closed { reason: Option<String> },
            }
        }
    }
}

#[test]
fn an_internally_tagged_unit_variant_gains_fields_like_an_object() {
    insta::assert_snapshot!(changes::<unit_variant::old::Event, unit_variant::new::Event>(), @"
    Compatible inout_test input body::Closed.reason: optional field added
    Compatible inout_test output body::Closed.reason: field added
    ");
}

mod variant_added {
    pub mod old {
        contract! {
            pub enum Closed {
                Active,
                Archived,
            }
            #[reflectapi(open)]
            #[serde(tag = "kind")]
            pub enum Open {
                Active,
                Archived,
            }
            pub enum CatchAll {
                Active,
                Archived,
                #[serde(other)]
                Unknown,
            }
            pub struct Pet {
                pub closed: Closed,
                pub open: Open,
                pub catch_all: CatchAll,
            }
        }
    }
    pub mod new {
        contract! {
            pub enum Closed {
                Active,
                Archived,
                Deleted,
            }
            #[reflectapi(open)]
            #[serde(tag = "kind")]
            pub enum Open {
                Active,
                Archived,
                Deleted,
            }
            pub enum CatchAll {
                Active,
                Archived,
                Deleted,
                #[serde(other)]
                Unknown,
            }
            pub struct Pet {
                pub closed: Closed,
                pub open: Open,
                pub catch_all: CatchAll,
            }
        }
    }
}

#[test]
fn a_new_response_variant_breaks_only_closed_enums() {
    insta::assert_snapshot!(changes::<variant_added::old::Pet, variant_added::new::Pet>(), @"
    Breaking inout_test output body.closed::Deleted: variant added, old clients can't read it
    Compatible inout_test input body.catch_all::Deleted: variant added
    Compatible inout_test input body.closed::Deleted: variant added
    Compatible inout_test input body.open::Deleted: variant added
    Compatible inout_test output body.catch_all::Deleted: variant added, old clients read it as unknown
    Compatible inout_test output body.open::Deleted: variant added, old clients read it as unknown
    ");
}

mod tag_renamed {
    pub mod old {
        contract! {
            #[serde(tag = "kind")]
            pub enum Shape {
                Circle { radius: u32 },
            }
        }
    }
    pub mod new {
        contract! {
            #[serde(tag = "type")]
            pub enum Shape {
                Circle { radius: u32 },
            }
        }
    }
}

#[test]
fn renaming_an_enum_tag_breaks_both_ways() {
    insta::assert_snapshot!(changes::<tag_renamed::old::Shape, tag_renamed::new::Shape>(), @"
    Breaking inout_test input body: changed from internally tagged by `kind` to internally tagged by `type`
    Breaking inout_test output body: changed from internally tagged by `kind` to internally tagged by `type`
    ");
}

mod generic_argument {
    pub mod old {
        contract! {
            pub struct Page<T> {
                pub items: Vec<T>,
                pub total: u32,
            }
            pub struct Pet {
                pub name: String,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Page<T> {
                pub items: Vec<T>,
                pub total: u32,
            }
            pub struct Pet {
                pub name: String,
            }
            pub struct PetWithPhoto {
                #[serde(flatten)]
                pub pet: Pet,
                pub photo_url: Option<String>,
            }
        }
    }
}

#[test]
fn a_generic_argument_extended_by_flattening_adds_its_fields() {
    insta::assert_snapshot!(changes::<
        generic_argument::old::Page<generic_argument::old::Pet>,
        generic_argument::new::Page<generic_argument::new::PetWithPhoto>,
    >(), @"
    Compatible inout_test input body.items[].photo_url: optional field added
    Compatible inout_test output body.items[].photo_url: field added
    ");
}

mod newtype {
    pub mod old {
        contract! {
            #[serde(transparent)]
            pub struct PetId(pub u32);
            pub struct Pet {
                pub id: PetId,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub id: u32,
            }
        }
    }
}

#[test]
fn a_transparent_newtype_is_the_value_it_wraps() {
    insta::assert_snapshot!(changes::<newtype::old::Pet, newtype::new::Pet>(), @"");
}

mod plain_newtype {
    pub mod old {
        contract! {
            pub struct PetId(pub u32);
            pub struct Pet {
                pub id: PetId,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub id: u32,
            }
        }
    }
}

#[test]
fn a_newtype_is_the_value_it_wraps_without_transparent() {
    insta::assert_snapshot!(changes::<plain_newtype::old::Pet, plain_newtype::new::Pet>(), @"");
    insta::assert_snapshot!(changes::<plain_newtype::new::Pet, plain_newtype::old::Pet>(), @"");
}

mod untagged_variant {
    pub mod old {
        contract! {
            pub enum Tag {
                Code(u32),
                Label(String),
            }
            pub struct Pet {
                pub tag: Tag,
            }
        }
    }
    pub mod new {
        contract! {
            pub enum Tag {
                Code(u32),
                #[serde(untagged)]
                Label(String),
            }
            pub struct Pet {
                pub tag: Tag,
            }
        }
    }
}

#[test]
fn a_variant_becoming_untagged_breaks_both_ways() {
    insta::assert_snapshot!(changes::<untagged_variant::old::Pet, untagged_variant::new::Pet>(), @"
    Breaking inout_test input body.tag::Label: now untagged, written as its content alone
    Breaking inout_test output body.tag::Label: now untagged, written as its content alone
    ");
    insta::assert_snapshot!(changes::<untagged_variant::new::Pet, untagged_variant::old::Pet>(), @"
    Breaking inout_test input body.tag::Label: no longer untagged, written with its tag
    Breaking inout_test output body.tag::Label: no longer untagged, written with its tag
    ");
}

mod primitives {
    pub mod old {
        contract! {
            pub struct Pet {
                pub id: uuid::Uuid,
                pub weight: u32,
            }
        }
    }
    pub mod new {
        contract! {
            pub struct Pet {
                pub id: String,
                pub weight: u64,
            }
        }
    }
}

#[test]
fn widening_a_value_breaks_responses_and_narrowing_breaks_requests() {
    insta::assert_snapshot!(changes::<primitives::old::Pet, primitives::new::Pet>(), @"
    Breaking inout_test output body.id: changed from `uuid::Uuid` to `std::string::String`, no longer only uuid
    Breaking inout_test output body.weight: changed from `u32` to `u64`
    Compatible inout_test input body.id: changed from `uuid::Uuid` to `std::string::String`, no longer only uuid
    Compatible inout_test input body.weight: changed from `u32` to `u64`
    ");
}

mod deprecated_field {
    pub mod announced {
        contract! {
            pub struct Pet {
                pub name: String,
                #[deprecated(note = "Use `name`.")]
                #[reflectapi(deprecated_since = "2026-10-01", sunset = "2026-11-01")]
                pub nickname: String,
            }
        }
    }
    pub mod unannounced {
        contract! {
            pub struct Pet {
                pub name: String,
                pub nickname: String,
            }
        }
    }
    pub mod removed {
        contract! {
            pub struct Pet {
                pub name: String,
            }
        }
    }
}

#[test]
fn a_deprecated_field_may_go_once_its_sunset_has_come() {
    use deprecated_field::{announced, removed, unannounced};

    let judged_on = |deprecated_in: Option<reflectapi::Schema>| {
        let mut options = Options::default();
        options.today = Some("2026-11-20".parse().unwrap());
        options.notice_cutoff = Some("2026-11-13".parse().unwrap());
        options.deprecated_in = deprecated_in;
        options
    };
    insta::assert_snapshot!(
        changes_with::<announced::Pet, removed::Pet>(&judged_on(None)),
        @"
    Compatible inout_test input body.nickname: field removed, the server ignores it if sent (deprecated, sunset 2026-11-01)
    Compatible inout_test output body.nickname: required field removed after its sunset 2026-11-01
    "
    );
    insta::assert_snapshot!(
        changes_with::<announced::Pet, removed::Pet>(&judged_on(Some(into_schema::<announced::Pet>()))),
        @"
    Compatible inout_test input body.nickname: field removed, the server ignores it if sent (deprecated, sunset 2026-11-01)
    Compatible inout_test output body.nickname: required field removed after its sunset 2026-11-01
    "
    );
    insta::assert_snapshot!(
        changes_with::<announced::Pet, removed::Pet>(&judged_on(Some(into_schema::<unannounced::Pet>()))),
        @"
    Breaking inout_test output body.nickname: required field removed, not deprecated in the earlier schema
    Compatible inout_test input body.nickname: field removed, the server ignores it if sent (deprecated, sunset 2026-11-01)
    "
    );
}
