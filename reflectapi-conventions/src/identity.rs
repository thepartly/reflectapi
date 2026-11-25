/// A request or response naming one entity: `{"identity": …}`.
///
/// For an entity that can be identified in more than one way, `I` saying
/// which, such as an enum of an ID or an external code, and for the items of
/// a bulk write. An entity with one fixed key takes it as a plain field, such
/// as `id`, instead.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
    reflectapi::Output,
)]
pub struct Identity<I>
where
    I: reflectapi::Input,
{
    pub identity: I,
}

/// An entity's identity beside its data, whose fields are flattened in:
/// `{"identity": …, "name": …}`.
///
/// For the same cases as [`Identity`]; with an `if` clause around it, it is
/// one item of a bulk write.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
    reflectapi::Output,
)]
pub struct IdentityData<I, D>
where
    I: reflectapi::Input,
    D: reflectapi::Input,
{
    pub identity: I,
    #[serde(flatten)]
    pub data: D,
}
