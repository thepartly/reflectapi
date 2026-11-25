//! The `conventions` types in a schema and every client.

use reflectapi::conventions::conflict::{self, actions, matching};
use reflectapi::conventions::{ForwardPaginatedList, Identity, IdentityData, PageRequest};

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
struct PetId(String);

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
struct PetData {
    name: String,
}

/// What a cursor holds: only the server reads it.
#[derive(serde::Serialize, serde::Deserialize)]
struct PetsCursor {
    after_id: u64,
}

async fn pets_list(
    _: (),
    _: PageRequest<reflectapi::Encoded<PetsCursor>>,
    _: reflectapi::Empty,
) -> ForwardPaginatedList<IdentityData<PetId, PetData>, reflectapi::Encoded<PetsCursor>> {
    unimplemented!()
}

async fn pets_insert(
    _: (),
    _: conflict::InsertOrElse<IdentityData<PetId, PetData>, actions::UpdateWithOrAbort<PetData>>,
    _: reflectapi::Empty,
) -> Identity<PetId> {
    unimplemented!()
}

async fn pets_replace_all(
    _: (),
    _: matching::InsertManyOrElse<PetData, actions::ReplaceOrAbort>,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

async fn pets_update(
    _: (),
    _: matching::UpdateOrElseRequired<IdentityData<PetId, PetData>, actions::Ignore>,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

async fn pets_remove(
    _: (),
    _: matching::RemoveOrElse<Identity<PetId>, actions::AbortOrIgnore>,
    _: reflectapi::Empty,
) -> reflectapi::Empty {
    unimplemented!()
}

#[test]
fn test_conventions() {
    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .route(pets_list, |b| b.name("pets.list"))
        .route(pets_insert, |b| b.name("pets.insert"))
        .route(pets_replace_all, |b| b.name("pets.replace-all"))
        .route(pets_update, |b| b.name("pets.update"))
        .route(pets_remove, |b| b.name("pets.remove")))
}
