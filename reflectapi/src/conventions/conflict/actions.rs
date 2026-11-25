//! The action sets: what a route lets a client choose when a write meets
//! what is stored, or doesn't find it.
//!
//! Each enum is one set of actions. The catalogue is every combination of
//! `abort`, `ignore` and `replace` with `update` or `update` with fields.
//! A set with `abort` defaults to it; a set without has no default, so the
//! client always chooses. Defaults are part of the convention, as is which
//! actions a route may offer together, so a route takes its set from here. On the wire an
//! action is its snake_case name, `"abort"`, or for one with data,
//! `{"update": {…}}`.
//!
//! `replace` and `update` both insert when the entity doesn't exist, and
//! differ in what an existing one keeps: `replace` overwrites it, resetting
//! the fields the request leaves out to their defaults, while `update`
//! patches it, so those fields keep their stored values. Insert with
//! `update` is the upsert.
//!
//! A set doesn't say which side it is for: the clause field it fills does.
//! [`AbortOrIgnore`] in `exists` is what an insert does when the entity
//! exists; in `not_exists`, what an update or remove does when it doesn't.
//!
//! | Set | Actions | Default | Used on | Idempotent |
//! |---|---|---|---|---|
//! | [`Abort`] | abort | abort | any | |
//! | [`Ignore`] | ignore | none | any | ✓ |
//! | [`AbortOrIgnore`] | abort, ignore | abort | any | |
//! | [`Replace`] | replace | none | insert, insert many | ✓ |
//! | [`ReplaceOrAbort`] | replace, abort | abort | insert, insert many | |
//! | [`ReplaceOrIgnore`] | replace, ignore | none | insert, insert many | ✓ |
//! | [`ReplaceOrElse`] | replace, abort, ignore | abort | insert, insert many | |
//! | [`Update`] | update | none | insert, insert many | ✓ |
//! | [`UpdateOrAbort`] | update, abort | abort | insert, insert many | |
//! | [`UpdateOrIgnore`] | update, ignore | none | insert, insert many | ✓ |
//! | [`UpdateOrElse`] | update, abort, ignore | abort | insert, insert many | |
//! | [`UpdateOrReplace`] | update, replace | none | insert, insert many | ✓ |
//! | [`UpdateOrReplaceOrAbort`] | update, replace, abort | abort | insert, insert many | |
//! | [`UpdateOrReplaceOrIgnore`] | update, replace, ignore | none | insert, insert many | ✓ |
//! | [`UpdateOrReplaceOrElse`] | update, replace, abort, ignore | abort | insert, insert many | |
//! | [`UpdateWith`] | update with fields | none | insert | ✓ |
//! | [`UpdateWithOrAbort`] | update with fields, abort | abort | insert | |
//! | [`UpdateWithOrIgnore`] | update with fields, ignore | none | insert | ✓ |
//! | [`UpdateWithOrElse`] | update with fields, abort, ignore | abort | insert | |
//! | [`UpdateWithOrReplace`] | update with fields, replace | none | insert | ✓ |
//! | [`UpdateWithOrReplaceOrAbort`] | update with fields, replace, abort | abort | insert | |
//! | [`UpdateWithOrReplaceOrIgnore`] | update with fields, replace, ignore | none | insert | ✓ |
//! | [`UpdateWithOrReplaceOrElse`] | update with fields, replace, abort, ignore | abort | insert | |
//!
//! "OrElse" means "or abort, the default, or ignore". A route whose set has
//! no default takes a `…OrElseRequired` alias, such as
//! [`InsertOrElseRequired`](super::InsertOrElseRequired), whose `if` is
//! required. An insert of several doesn't update with fields, since each item
//! would need its own. An update or remove offers only `abort` and `ignore`:
//! there is nothing to replace or update.
//!
//! ```
//! use reflectapi::conventions::conflict::{actions, InsertOrElse, RemoveOrElse};
//!
//! #[derive(serde::Deserialize, reflectapi::Input)]
//! struct Pet {
//!     name: String,
//! }
//!
//! // If it exists: replace or abort.
//! let insert: InsertOrElse<Pet, actions::ReplaceOrAbort> =
//!     serde_json::from_str(r#"{"if": {"exists": "replace"}, "name": "Rex"}"#).unwrap();
//! assert_eq!(insert.if_.exists, actions::ReplaceOrAbort::Replace);
//!
//! // If it doesn't exist: abort or ignore.
//! let remove: RemoveOrElse<Pet, actions::AbortOrIgnore> =
//!     serde_json::from_str(r#"{"if": {"not_exists": "ignore"}, "name": "Rex"}"#).unwrap();
//! assert_eq!(remove.if_.not_exists, actions::AbortOrIgnore::Ignore);
//! ```

/// An action set every action of which ends in the same state when the same
/// write is processed again: it has no `abort`. A write the server may
/// process more than once, such as from a queue, offers only these:
///
/// ```
/// use reflectapi::conventions::conflict::actions::{Idempotent, UpdateOrReplaceOrIgnore};
/// use reflectapi::conventions::conflict::InsertOrElseRequired;
///
/// #[derive(serde::Deserialize, reflectapi::Input)]
/// struct Pet {
///     name: String,
/// }
///
/// /// Takes items from a queue that may deliver one more than once.
/// fn consume<C: Idempotent + reflectapi::Input>(_: InsertOrElseRequired<Pet, C>) {}
///
/// let item = serde_json::from_str(r#"{"if": {"exists": "ignore"}, "name": "Rex"}"#).unwrap();
/// consume::<UpdateOrReplaceOrIgnore>(item);
/// ```
///
/// It covers the actions only. A [`matching`](super::matching) edition
/// condition fails when the same write comes again, since the first attempt
/// changed the edition, and effects of a write, such as an event it sends,
/// happen again.
pub trait Idempotent {}

/// Declares action sets with the derives they share. A set with a default
/// also derives `Default`, with the default variant marked `#[default]`.
macro_rules! action_sets {
    ($($(#[$meta:meta])* $name:ident $(<$param:ident>)? { $($variants:tt)* })+) => {$(
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash,
            serde::Serialize, serde::Deserialize, crate::Input,
        )]
        #[serde(rename_all = "snake_case")]
        pub enum $name $(<$param>)? { $($variants)* }
    )+};
}

action_sets! {
    #[derive(Default)]
    Abort { #[default] Abort }
    Ignore { Ignore }
    #[derive(Default)]
    AbortOrIgnore { #[default] Abort, Ignore }
    Replace { Replace }
    #[derive(Default)]
    ReplaceOrAbort { Replace, #[default] Abort }
    ReplaceOrIgnore { Replace, Ignore }
    #[derive(Default)]
    ReplaceOrElse { Replace, #[default] Abort, Ignore }
    Update { Update }
    #[derive(Default)]
    UpdateOrAbort { Update, #[default] Abort }
    UpdateOrIgnore { Update, Ignore }
    #[derive(Default)]
    UpdateOrElse { Update, #[default] Abort, Ignore }
    UpdateOrReplace { Update, Replace }
    #[derive(Default)]
    UpdateOrReplaceOrAbort { Update, Replace, #[default] Abort }
    UpdateOrReplaceOrIgnore { Update, Replace, Ignore }
    #[derive(Default)]
    UpdateOrReplaceOrElse { Update, Replace, #[default] Abort, Ignore }
    UpdateWith<T> { Update(T) }
    #[derive(Default)]
    UpdateWithOrAbort<T> { Update(T), #[default] Abort }
    UpdateWithOrIgnore<T> { Update(T), Ignore }
    #[derive(Default)]
    UpdateWithOrElse<T> { Update(T), #[default] Abort, Ignore }
    UpdateWithOrReplace<T> { Update(T), Replace }
    #[derive(Default)]
    UpdateWithOrReplaceOrAbort<T> { Update(T), Replace, #[default] Abort }
    UpdateWithOrReplaceOrIgnore<T> { Update(T), Replace, Ignore }
    #[derive(Default)]
    UpdateWithOrReplaceOrElse<T> { Update(T), Replace, #[default] Abort, Ignore }
}

impl Idempotent for Ignore {}
impl Idempotent for Replace {}
impl Idempotent for ReplaceOrIgnore {}
impl Idempotent for Update {}
impl Idempotent for UpdateOrIgnore {}
impl Idempotent for UpdateOrReplace {}
impl Idempotent for UpdateOrReplaceOrIgnore {}
impl<T> Idempotent for UpdateWith<T> {}
impl<T> Idempotent for UpdateWithOrIgnore<T> {}
impl<T> Idempotent for UpdateWithOrReplace<T> {}
impl<T> Idempotent for UpdateWithOrReplaceOrIgnore<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalogue is the convention: changing a set's actions or default
    /// is a change here too.
    #[test]
    fn the_catalogue() {
        fn actions<S: crate::Input>() -> Vec<String> {
            let mut schema = crate::Typespace::new();
            let set = S::reflectapi_input_type(&mut schema);
            let set = schema.get_type(&set.name).unwrap().as_enum().unwrap();
            set.variants().map(|v| v.serde_name().to_owned()).collect()
        }
        fn default<S: Default + serde::Serialize>() -> String {
            serde_json::to_value(S::default())
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        }
        type T = u8;
        let catalogue: [(Vec<String>, &[&str]); 23] = [
            (actions::<Abort>(), &["abort"]),
            (actions::<Ignore>(), &["ignore"]),
            (actions::<AbortOrIgnore>(), &["abort", "ignore"]),
            (actions::<Replace>(), &["replace"]),
            (actions::<ReplaceOrAbort>(), &["replace", "abort"]),
            (actions::<ReplaceOrIgnore>(), &["replace", "ignore"]),
            (actions::<ReplaceOrElse>(), &["replace", "abort", "ignore"]),
            (actions::<Update>(), &["update"]),
            (actions::<UpdateOrAbort>(), &["update", "abort"]),
            (actions::<UpdateOrIgnore>(), &["update", "ignore"]),
            (actions::<UpdateOrElse>(), &["update", "abort", "ignore"]),
            (actions::<UpdateOrReplace>(), &["update", "replace"]),
            (
                actions::<UpdateOrReplaceOrAbort>(),
                &["update", "replace", "abort"],
            ),
            (
                actions::<UpdateOrReplaceOrIgnore>(),
                &["update", "replace", "ignore"],
            ),
            (
                actions::<UpdateOrReplaceOrElse>(),
                &["update", "replace", "abort", "ignore"],
            ),
            (actions::<UpdateWith<T>>(), &["update"]),
            (actions::<UpdateWithOrAbort<T>>(), &["update", "abort"]),
            (actions::<UpdateWithOrIgnore<T>>(), &["update", "ignore"]),
            (
                actions::<UpdateWithOrElse<T>>(),
                &["update", "abort", "ignore"],
            ),
            (actions::<UpdateWithOrReplace<T>>(), &["update", "replace"]),
            (
                actions::<UpdateWithOrReplaceOrAbort<T>>(),
                &["update", "replace", "abort"],
            ),
            (
                actions::<UpdateWithOrReplaceOrIgnore<T>>(),
                &["update", "replace", "ignore"],
            ),
            (
                actions::<UpdateWithOrReplaceOrElse<T>>(),
                &["update", "replace", "abort", "ignore"],
            ),
        ];
        for (actual, expected) in catalogue {
            assert_eq!(actual, expected);
        }

        // The defaults. The sets left out have none, and don't implement
        // `Default`.
        let defaults = [
            (default::<Abort>(), "abort"),
            (default::<AbortOrIgnore>(), "abort"),
            (default::<ReplaceOrAbort>(), "abort"),
            (default::<ReplaceOrElse>(), "abort"),
            (default::<UpdateOrAbort>(), "abort"),
            (default::<UpdateOrElse>(), "abort"),
            (default::<UpdateOrReplaceOrAbort>(), "abort"),
            (default::<UpdateOrReplaceOrElse>(), "abort"),
            (default::<UpdateWithOrAbort<T>>(), "abort"),
            (default::<UpdateWithOrElse<T>>(), "abort"),
            (default::<UpdateWithOrReplaceOrAbort<T>>(), "abort"),
            (default::<UpdateWithOrReplaceOrElse<T>>(), "abort"),
        ];
        for (actual, expected) in defaults {
            assert_eq!(actual, expected);
        }

        // The idempotent sets: those without `abort`.
        fn idempotent<S: Idempotent>() {}
        idempotent::<Ignore>();
        idempotent::<Replace>();
        idempotent::<ReplaceOrIgnore>();
        idempotent::<Update>();
        idempotent::<UpdateOrIgnore>();
        idempotent::<UpdateOrReplace>();
        idempotent::<UpdateOrReplaceOrIgnore>();
        idempotent::<UpdateWith<T>>();
        idempotent::<UpdateWithOrIgnore<T>>();
        idempotent::<UpdateWithOrReplace<T>>();
        idempotent::<UpdateWithOrReplaceOrIgnore<T>>();
    }
}
