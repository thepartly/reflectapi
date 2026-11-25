//! The [`conflict`](super) clauses with a match condition: before writing,
//! the server checks the entity, or for a write of several, their parent,
//! and aborts on a mismatch. `{"if": {"exists": "replace", "matches":
//! {"edition": 3}}, …}` replaces only an entity at edition 3.
//!
//! The [`actions`](super::actions), [`Conditional`] and
//! [`ConditionalRequired`] are the [`conflict`](super) module's. Each
//! `…OrElse` alias has a `…OrElseRequired` twin whose action is required,
//! for a set without a default.
//! An edition condition fails when the same write comes again, so these
//! clauses don't make a write idempotent.
//!
//! ```
//! use reflectapi_conventions::conflict::matching::{self, EditionMatchCondition};
//! use reflectapi_conventions::conflict::actions;
//!
//! #[derive(serde::Deserialize, reflectapi::Input)]
//! struct PetName {
//!     name: String,
//! }
//!
//! // Remove Rex only if it is still at edition 7, the one the client read.
//! let remove: matching::RemoveOrElse<PetName, actions::Abort> = serde_json::from_str(
//!     r#"{"if": {"matches": {"edition": 7}}, "name": "Rex"}"#,
//! )
//! .unwrap();
//! assert_eq!(remove.if_.matches, Some(EditionMatchCondition::Edition(7)));
//! assert_eq!(remove.if_.not_exists, actions::Abort::Abort);
//! ```

use super::{Conditional, ConditionalRequired};

/// An insert, checked against `matches` first.
pub type InsertOrElse<T, C> = Conditional<T, IfConflictOnInsert<C>>;
/// An insert of several entities, checked against the parent first.
pub type InsertManyOrElse<T, C> = Conditional<T, IfConflictOnInsertMany<C>>;
/// A remove, checked against `matches` first.
pub type RemoveOrElse<T, C> = Conditional<T, IfConflictOnRemove<C>>;
/// A remove of several entities, checked against the parent first.
pub type RemoveManyOrElse<T, C> = Conditional<T, IfConflictOnRemoveMany<C>>;
/// An update, checked against `matches` first.
pub type UpdateOrElse<T, C> = Conditional<T, IfConflictOnUpdate<C>>;
/// An update of several entities, checked against the parent first.
pub type UpdateManyOrElse<T, C> = Conditional<T, IfConflictOnUpdateMany<C>>;

/// [`InsertOrElse`] whose `if` and `exists` are required.
pub type InsertOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertRequired<C>>;
/// [`InsertManyOrElse`] whose `if` and `exists` are required.
pub type InsertManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertManyRequired<C>>;
/// [`RemoveOrElse`] whose `if` and `not_exists` are required.
pub type RemoveOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveRequired<C>>;
/// [`RemoveManyOrElse`] whose `if` and `not_exists` are required.
pub type RemoveManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveManyRequired<C>>;
/// [`UpdateOrElse`] whose `if` and `not_exists` are required.
pub type UpdateOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateRequired<C>>;
/// [`UpdateManyOrElse`] whose `if` and `not_exists` are required.
pub type UpdateManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateManyRequired<C>>;

/// What the checked entity must be: `exists`, or `{"edition": n}`, existing
/// at edition `n`.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
)]
#[serde(rename_all = "snake_case")]
pub enum MatchCondition {
    #[default]
    Exists,
    Edition(u64),
}

impl MatchCondition {
    /// Whether this is `exists`, the default, which serializing leaves out.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Exists)
    }
}

/// A [`MatchCondition`] that can only be `exists`.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
)]
#[serde(rename_all = "snake_case")]
pub enum ExistenceMatchCondition {
    #[default]
    Exists,
}

impl ExistenceMatchCondition {
    /// Always true: `exists` is its only value.
    pub fn is_default(&self) -> bool {
        true
    }
}

/// A [`MatchCondition`] that can only be `{"edition": n}`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
#[serde(rename_all = "snake_case")]
pub enum EditionMatchCondition {
    Edition(u64),
}

impl From<ExistenceMatchCondition> for MatchCondition {
    fn from(condition: ExistenceMatchCondition) -> Self {
        match condition {
            ExistenceMatchCondition::Exists => Self::Exists,
        }
    }
}

impl From<EditionMatchCondition> for MatchCondition {
    fn from(condition: EditionMatchCondition) -> Self {
        match condition {
            EditionMatchCondition::Edition(edition) => Self::Edition(edition),
        }
    }
}

impl TryFrom<MatchCondition> for ExistenceMatchCondition {
    type Error = MatchCondition;

    /// `exists`, or the condition back if it is an edition.
    fn try_from(condition: MatchCondition) -> Result<Self, Self::Error> {
        match condition {
            MatchCondition::Exists => Ok(Self::Exists),
            other => Err(other),
        }
    }
}

impl TryFrom<MatchCondition> for EditionMatchCondition {
    type Error = MatchCondition;

    /// The edition, or the condition back if it is `exists`.
    fn try_from(condition: MatchCondition) -> Result<Self, Self::Error> {
        match condition {
            MatchCondition::Edition(edition) => Ok(Self::Edition(edition)),
            other => Err(other),
        }
    }
}

/// The `if` of an insert: the action when the entity exists, and what it
/// must match, if anything.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnInsert<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<MatchCondition>,
}

/// The `if` of an insert of several entities: the action for each listed
/// item that exists, and what the parent must match, `exists` by default.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnInsertMany<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}

/// The `if` of a remove: the action when the entity doesn't exist, and the
/// edition it must be at, if any.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnRemove<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<EditionMatchCondition>,
}

/// The `if` of a remove of several entities: the action when some don't
/// exist, and what the parent must match, `exists` by default.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnRemoveMany<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}

/// The `if` of an update: the action when the entity doesn't exist, and the
/// edition it must be at, if any.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnUpdate<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<EditionMatchCondition>,
}

/// The `if` of an update of several entities: the action when some don't
/// exist, and what the parent must match, `exists` by default.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnUpdateMany<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}

/// The `if` of [`InsertOrElseRequired`]: [`IfConflictOnInsert`] with `exists`
/// required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnInsertRequired<C>
where
    C: reflectapi::Input,
{
    pub exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<MatchCondition>,
}

/// The `if` of [`InsertManyOrElseRequired`]: [`IfConflictOnInsertMany`] with
/// `exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnInsertManyRequired<C>
where
    C: reflectapi::Input,
{
    pub exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}

/// The `if` of [`RemoveOrElseRequired`]: [`IfConflictOnRemove`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnRemoveRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<EditionMatchCondition>,
}

/// The `if` of [`RemoveManyOrElseRequired`]: [`IfConflictOnRemoveMany`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnRemoveManyRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}

/// The `if` of [`UpdateOrElseRequired`]: [`IfConflictOnUpdate`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnUpdateRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<EditionMatchCondition>,
}

/// The `if` of [`UpdateManyOrElseRequired`]: [`IfConflictOnUpdateMany`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnUpdateManyRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
    #[serde(default, skip_serializing_if = "MatchCondition::is_default")]
    pub parent_matches: MatchCondition,
}
