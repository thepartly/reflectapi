//! What a write does when it meets what is stored, or doesn't find it: an
//! `if` clause beside the request's own fields.
//!
//! ```json
//! {"if": {"exists": "replace"}, "name": "Rex"}
//! ```
//!
//! An insert says what to do when the entity exists, an update or remove
//! what to do when it doesn't. Without `if`, or without one of its fields,
//! the server takes the default. The clauses here check nothing else; those in
//! [`matching`] also check the entity's existence or edition first.
//!
//! # A route
//!
//! A route's input is one of the aliases, such as [`InsertOrElse`], of its
//! data and the set of actions it offers. The handler matches on the action:
//!
//! ```
//! use reflectapi_conventions::conflict::{actions, InsertOrElse};
//!
//! #[derive(serde::Deserialize, reflectapi::Input)]
//! struct Pet {
//!     name: String,
//! }
//!
//! #[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
//! enum PetsCreateError {
//!     #[reflectapi(status = 409)]
//!     Exists,
//! }
//!
//! async fn pets_create(
//!     _: (),
//!     input: InsertOrElse<Pet, actions::ReplaceOrAbort>,
//!     _: reflectapi::Empty,
//! ) -> Result<reflectapi::Empty, PetsCreateError> {
//!     let stored = false; // whether a pet named `input.data.name` exists
//!     if stored {
//!         match input.if_.exists {
//!             actions::ReplaceOrAbort::Replace => {} // write `input.data` over it
//!             actions::ReplaceOrAbort::Abort => return Err(PetsCreateError::Exists),
//!         }
//!     }
//!     Ok(reflectapi::Empty {})
//! }
//!
//! let (schema, _) = reflectapi::Builder::<()>::new()
//!     .route(pets_create, |b| b.name("pets.create"))
//!     .build()
//!     .unwrap();
//! ```
//!
//! The route's schema offers only the set's actions, and a request with any
//! other is refused:
//!
//! ```
//! # use reflectapi_conventions::conflict::{actions, InsertOrElse};
//! # #[derive(Debug, PartialEq, serde::Deserialize, reflectapi::Input)]
//! # struct Pet {
//! #     name: String,
//! # }
//! type PetsCreate = InsertOrElse<Pet, actions::ReplaceOrAbort>;
//!
//! let insert: PetsCreate =
//!     serde_json::from_str(r#"{"if": {"exists": "replace"}, "name": "Rex"}"#).unwrap();
//! assert_eq!(insert.if_.exists, actions::ReplaceOrAbort::Replace);
//! assert_eq!(insert.data, Pet { name: "Rex".into() });
//!
//! // Without `if`, the default: abort.
//! let insert: PetsCreate = serde_json::from_str(r#"{"name": "Rex"}"#).unwrap();
//! assert_eq!(insert.if_.exists, actions::ReplaceOrAbort::Abort);
//!
//! // `ignore` isn't in the set.
//! assert!(serde_json::from_str::<PetsCreate>(r#"{"if": {"exists": "ignore"}, "name": "Rex"}"#)
//!     .is_err());
//! ```
//!
//! An update or remove says what to do when the entity doesn't exist:
//!
//! ```
//! use reflectapi_conventions::conflict::{actions, RemoveOrElse};
//!
//! #[derive(serde::Deserialize, reflectapi::Input)]
//! struct PetName {
//!     name: String,
//! }
//!
//! let remove: RemoveOrElse<PetName, actions::AbortOrIgnore> =
//!     serde_json::from_str(r#"{"if": {"not_exists": "ignore"}, "name": "Rex"}"#).unwrap();
//! assert_eq!(remove.if_.not_exists, actions::AbortOrIgnore::Ignore);
//! ```
//!
//! # The action sets
//!
//! [`actions`] is the catalogue of sets a route chooses from, such as
//! [`actions::ReplaceOrAbort`], with its default, if it has one. A set
//! doesn't say which side it is for: the clause field it fills, `exists` or
//! `not_exists`, does.
//!
//! # Processing a write again
//!
//! [`actions::Idempotent`] marks the sets without `abort`, which end in the
//! same state when the same write comes again.

pub mod actions;
pub mod matching;

/// An insert of one entity or several, with what to do when one exists.
/// For several, `exists` decides each listed item.
pub type InsertOrElse<T, C> = Conditional<T, IfConflictOnInsert<C>>;
/// A remove of one entity or several, with what to do when one doesn't
/// exist. For several, `ignore` removes the rest and `abort` aborts them all.
pub type RemoveOrElse<T, C> = Conditional<T, IfConflictOnRemove<C>>;
/// An update of one entity or several, with what to do when one doesn't
/// exist. For several, `ignore` updates the rest and `abort` aborts them all.
pub type UpdateOrElse<T, C> = Conditional<T, IfConflictOnUpdate<C>>;

/// [`InsertOrElse`] whose `if` and `exists` are required.
pub type InsertOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertRequired<C>>;
/// [`RemoveOrElse`] whose `if` and `not_exists` are required.
pub type RemoveOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveRequired<C>>;
/// [`UpdateOrElse`] whose `if` and `not_exists` are required.
pub type UpdateOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateRequired<C>>;

/// A write of `data` with an `if` clause, whose absence takes the clause's
/// defaults: `{"if": {…}, …data}`. The aliases above, such as
/// [`InsertOrElse`], name its usual clauses.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct Conditional<T, If>
where
    T: reflectapi::Input,
    If: reflectapi::Input,
{
    #[serde(rename = "if", default)]
    pub if_: If,
    #[serde(flatten)]
    pub data: T,
}

/// A write of `data` whose `if` is required, as is the action in it:
/// `{"if": {…}, …data}`. A route whose set has no default takes it, through
/// one of the aliases above, such as [`InsertOrElseRequired`].
///
/// ```
/// use reflectapi_conventions::conflict::{actions, InsertOrElseRequired, RemoveOrElseRequired};
///
/// #[derive(serde::Deserialize, reflectapi::Input)]
/// struct Pet {
///     name: String,
/// }
///
/// type PetsImport = InsertOrElseRequired<Pet, actions::ReplaceOrIgnore>;
///
/// let insert: PetsImport =
///     serde_json::from_str(r#"{"if": {"exists": "ignore"}, "name": "Rex"}"#).unwrap();
/// assert_eq!(insert.if_.exists, actions::ReplaceOrIgnore::Ignore);
///
/// // Without `if`, refused.
/// assert!(serde_json::from_str::<PetsImport>(r#"{"name": "Rex"}"#).is_err());
///
/// // A remove that only ignores a missing pet: the client says so.
/// let remove: RemoveOrElseRequired<Pet, actions::Ignore> =
///     serde_json::from_str(r#"{"if": {"not_exists": "ignore"}, "name": "Rex"}"#).unwrap();
/// assert_eq!(remove.if_.not_exists, actions::Ignore::Ignore);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct ConditionalRequired<T, If>
where
    T: reflectapi::Input,
    If: reflectapi::Input,
{
    #[serde(rename = "if")]
    pub if_: If,
    #[serde(flatten)]
    pub data: T,
}

/// The `if` of an insert, of one entity or several.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnInsert<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub exists: C,
}

/// The `if` of [`InsertOrElseRequired`]: [`IfConflictOnInsert`] with `exists`
/// required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnInsertRequired<C>
where
    C: reflectapi::Input,
{
    pub exists: C,
}

/// The `if` of a remove, of one entity or several.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnRemove<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
}

/// The `if` of [`RemoveOrElseRequired`]: [`IfConflictOnRemove`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnRemoveRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
}

/// The `if` of an update, of one entity or several.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input,
)]
pub struct IfConflictOnUpdate<C>
where
    C: reflectapi::Input,
{
    #[serde(default)]
    pub not_exists: C,
}

/// The `if` of [`UpdateOrElseRequired`]: [`IfConflictOnUpdate`] with
/// `not_exists` required.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct IfConflictOnUpdateRequired<C>
where
    C: reflectapi::Input,
{
    pub not_exists: C,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
    struct Pet {
        name: String,
    }

    #[test]
    fn an_absent_if_takes_the_defaults() {
        let insert: InsertOrElse<Pet, actions::ReplaceOrAbort> =
            serde_json::from_str(r#"{"name":"Rex"}"#).unwrap();
        assert_eq!(insert.if_.exists, actions::ReplaceOrAbort::Abort);
        assert_eq!(insert.data, Pet { name: "Rex".into() });
    }

    #[test]
    fn the_if_sits_beside_the_flattened_data() {
        let insert: InsertOrElse<Pet, actions::ReplaceOrAbort> =
            serde_json::from_str(r#"{"if":{"exists":"replace"},"name":"Rex"}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&insert).unwrap(),
            r#"{"if":{"exists":"replace"},"name":"Rex"}"#
        );
    }

    #[test]
    fn an_action_the_set_does_not_have_is_refused() {
        for json in [
            r#"{"if":{"exists":"ignore"},"name":"Rex"}"#,
            r#"{"if":{"exists":"replace_all"},"name":"Rex"}"#,
        ] {
            let error = serde_json::from_str::<InsertOrElse<Pet, actions::ReplaceOrAbort>>(json)
                .unwrap_err();
            assert!(error.to_string().contains("unknown variant"), "{error}");
        }
    }

    #[test]
    fn an_update_with_fields_carries_them() {
        let insert: InsertOrElse<Pet, actions::UpdateWithOrAbort<Pet>> =
            serde_json::from_str(r#"{"if":{"exists":{"update":{"name":"Max"}}},"name":"Rex"}"#)
                .unwrap();
        assert_eq!(
            insert.if_.exists,
            actions::UpdateWithOrAbort::Update(Pet { name: "Max".into() })
        );
    }

    #[test]
    fn matching_checks_an_edition() {
        let insert: matching::InsertOrElse<Pet, actions::ReplaceOrAbort> = serde_json::from_str(
            r#"{"if":{"exists":"replace","matches":{"edition":3}},"name":"Rex"}"#,
        )
        .unwrap();
        assert_eq!(
            insert.if_.matches,
            Some(matching::MatchCondition::Edition(3))
        );

        // The parent must exist by default, which serializing leaves out.
        let insert: matching::InsertManyOrElse<Pet, actions::Abort> =
            serde_json::from_str(r#"{"name":"Rex"}"#).unwrap();
        assert_eq!(insert.if_.parent_matches, matching::MatchCondition::Exists);
        assert_eq!(
            serde_json::to_string(&insert).unwrap(),
            r#"{"if":{"exists":"abort"},"name":"Rex"}"#
        );
    }

    /// Every `…OrElseRequired` takes a set without a default, and refuses a
    /// request without `if` or without the action in it.
    #[test]
    fn with_condition_requires_the_action() {
        fn check<W: serde::de::DeserializeOwned>(action: &str) {
            let with_action = format!(r#"{{"if":{action},"name":"Rex"}}"#);
            assert!(
                serde_json::from_str::<W>(&with_action).is_ok(),
                "{with_action}"
            );
            for json in [r#"{"name":"Rex"}"#, r#"{"if":{},"name":"Rex"}"#] {
                assert!(serde_json::from_str::<W>(json).is_err(), "{json}");
            }
        }
        let exists = r#"{"exists":"ignore"}"#;
        let not_exists = r#"{"not_exists":"ignore"}"#;
        check::<InsertOrElseRequired<Pet, actions::ReplaceOrIgnore>>(exists);
        check::<RemoveOrElseRequired<Pet, actions::Ignore>>(not_exists);
        check::<UpdateOrElseRequired<Pet, actions::Ignore>>(not_exists);
        check::<matching::InsertOrElseRequired<Pet, actions::ReplaceOrIgnore>>(exists);
        check::<matching::InsertManyOrElseRequired<Pet, actions::UpdateOrIgnore>>(exists);
        check::<matching::RemoveOrElseRequired<Pet, actions::Ignore>>(not_exists);
        check::<matching::RemoveManyOrElseRequired<Pet, actions::Ignore>>(not_exists);
        check::<matching::UpdateOrElseRequired<Pet, actions::Ignore>>(not_exists);
        check::<matching::UpdateManyOrElseRequired<Pet, actions::Ignore>>(not_exists);
    }

    #[test]
    fn match_conditions_convert() {
        use matching::{EditionMatchCondition, ExistenceMatchCondition, MatchCondition};
        assert_eq!(
            MatchCondition::from(EditionMatchCondition::Edition(2)),
            MatchCondition::Edition(2)
        );
        assert_eq!(
            MatchCondition::from(ExistenceMatchCondition::Exists),
            MatchCondition::Exists
        );
        assert_eq!(
            EditionMatchCondition::try_from(MatchCondition::Exists),
            Err(MatchCondition::Exists)
        );
    }
}
