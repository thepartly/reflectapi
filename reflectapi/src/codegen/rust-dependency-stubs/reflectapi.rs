#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Option<T> {
    #[default]
    Undefined,
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Empty {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Infallible {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bounded<T, const MIN: usize, const MAX: usize>(T);

/// Stubs of the `conventions` feature's types.
pub mod conventions {
    pub mod pagination {
        pub struct PageRequest<C = String> {
            pub cursor: Option<C>,
            pub limit: Option<std::num::NonZeroU32>,
        }

        pub struct NonPaginatedList<D> {
            pub items: Vec<D>,
        }

        pub struct ForwardPaginatedList<D, C = String> {
            pub items: Vec<D>,
            pub pagination: Option<ForwardPagination<C>>,
        }

        pub struct ForwardPagination<C = String> {
            pub next_cursor: Option<C>,
        }
    }

    pub mod identity {
        pub struct Identity<I> {
            pub identity: I,
        }

        pub struct IdentityData<I, D> {
            pub identity: I,
            pub data: D,
        }
    }

    pub mod conflict {
        pub type InsertOrElse<T, C> = Conditional<T, IfConflictOnInsert<C>>;
        pub type RemoveOrElse<T, C> = Conditional<T, IfConflictOnRemove<C>>;
        pub type UpdateOrElse<T, C> = Conditional<T, IfConflictOnUpdate<C>>;

        pub type InsertOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertRequired<C>>;
        pub type RemoveOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveRequired<C>>;
        pub type UpdateOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateRequired<C>>;

        pub struct Conditional<T, If> {
            pub if_: If,
            pub data: T,
        }

        pub struct ConditionalRequired<T, If> {
            pub if_: If,
            pub data: T,
        }

        pub struct IfConflictOnInsert<C> {
            pub exists: C,
        }

        pub struct IfConflictOnInsertRequired<C> {
            pub exists: C,
        }

        pub struct IfConflictOnRemove<C> {
            pub not_exists: C,
        }

        pub struct IfConflictOnUpdate<C> {
            pub not_exists: C,
        }

        pub struct IfConflictOnRemoveRequired<C> {
            pub not_exists: C,
        }

        pub struct IfConflictOnUpdateRequired<C> {
            pub not_exists: C,
        }

        pub mod actions {
            pub enum Abort { Abort }
            pub enum Ignore { Ignore }
            pub enum AbortOrIgnore { Abort, Ignore }
            pub enum Replace { Replace }
            pub enum ReplaceOrAbort { Replace, Abort }
            pub enum ReplaceOrIgnore { Replace, Ignore }
            pub enum ReplaceOrElse { Replace, Abort, Ignore }
            pub enum Update { Update }
            pub enum UpdateOrAbort { Update, Abort }
            pub enum UpdateOrIgnore { Update, Ignore }
            pub enum UpdateOrElse { Update, Abort, Ignore }
            pub enum UpdateOrReplace { Update, Replace }
            pub enum UpdateOrReplaceOrAbort { Update, Replace, Abort }
            pub enum UpdateOrReplaceOrIgnore { Update, Replace, Ignore }
            pub enum UpdateOrReplaceOrElse { Update, Replace, Abort, Ignore }
            pub enum UpdateWith<T> { Update(T) }
            pub enum UpdateWithOrAbort<T> { Update(T), Abort }
            pub enum UpdateWithOrIgnore<T> { Update(T), Ignore }
            pub enum UpdateWithOrElse<T> { Update(T), Abort, Ignore }
            pub enum UpdateWithOrReplace<T> { Update(T), Replace }
            pub enum UpdateWithOrReplaceOrAbort<T> { Update(T), Replace, Abort }
            pub enum UpdateWithOrReplaceOrIgnore<T> { Update(T), Replace, Ignore }
            pub enum UpdateWithOrReplaceOrElse<T> { Update(T), Replace, Abort, Ignore }
        }

        pub mod matching {
            use super::{Conditional, ConditionalRequired};

            pub type InsertOrElse<T, C> = Conditional<T, IfConflictOnInsert<C>>;
            pub type InsertManyOrElse<T, C> = Conditional<T, IfConflictOnInsertMany<C>>;
            pub type RemoveOrElse<T, C> = Conditional<T, IfConflictOnRemove<C>>;
            pub type RemoveManyOrElse<T, C> = Conditional<T, IfConflictOnRemoveMany<C>>;
            pub type UpdateOrElse<T, C> = Conditional<T, IfConflictOnUpdate<C>>;
            pub type UpdateManyOrElse<T, C> = Conditional<T, IfConflictOnUpdateMany<C>>;

            pub type InsertOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertRequired<C>>;
            pub type InsertManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnInsertManyRequired<C>>;
            pub type RemoveOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveRequired<C>>;
            pub type RemoveManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnRemoveManyRequired<C>>;
            pub type UpdateOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateRequired<C>>;
            pub type UpdateManyOrElseRequired<T, C> = ConditionalRequired<T, IfConflictOnUpdateManyRequired<C>>;

            pub enum MatchCondition {
                Exists,
                Edition(u64),
            }

            pub enum ExistenceMatchCondition {
                Exists,
            }

            pub enum EditionMatchCondition {
                Edition(u64),
            }

            pub struct IfConflictOnInsert<C> {
                pub exists: C,
                pub matches: Option<MatchCondition>,
            }

            pub struct IfConflictOnInsertMany<C> {
                pub exists: C,
                pub parent_matches: MatchCondition,
            }

            pub struct IfConflictOnRemove<C> {
                pub not_exists: C,
                pub matches: Option<EditionMatchCondition>,
            }

            pub struct IfConflictOnRemoveMany<C> {
                pub not_exists: C,
                pub parent_matches: MatchCondition,
            }

            pub struct IfConflictOnUpdate<C> {
                pub not_exists: C,
                pub matches: Option<EditionMatchCondition>,
            }

            pub struct IfConflictOnUpdateMany<C> {
                pub not_exists: C,
                pub parent_matches: MatchCondition,
            }

            pub struct IfConflictOnInsertRequired<C> {
                pub exists: C,
                pub matches: Option<MatchCondition>,
            }

            pub struct IfConflictOnInsertManyRequired<C> {
                pub exists: C,
                pub parent_matches: MatchCondition,
            }

            pub struct IfConflictOnRemoveRequired<C> {
                pub not_exists: C,
                pub matches: Option<EditionMatchCondition>,
            }

            pub struct IfConflictOnRemoveManyRequired<C> {
                pub not_exists: C,
                pub parent_matches: MatchCondition,
            }

            pub struct IfConflictOnUpdateRequired<C> {
                pub not_exists: C,
                pub matches: Option<EditionMatchCondition>,
            }

            pub struct IfConflictOnUpdateManyRequired<C> {
                pub not_exists: C,
                pub parent_matches: MatchCondition,
            }
        }
    }
}

pub mod rt;
