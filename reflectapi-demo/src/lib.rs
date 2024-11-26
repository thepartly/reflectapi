use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub fn builder() -> reflectapi::Builder<Arc<AppState>> {
    // build an application, by providing pointers to handlers
    // and assigning those to named routes
    reflectapi::Builder::new()
        .name("Demo application")
        .description("This is a demo application")
        .route(health_check, |b| {
            b.name("health.check")
                .readonly(true)
                .tag("internal")
                .description("Check the health of the service")
        })
        .route(pets_list, |b| {
            b.name("pets.list")
                .readonly(true)
                .description("List available pets")
        })
        .route(pets_create, |b| {
            b.name("pets.create")
                .description("Create a new pet")
        })
        .route(pets_update, |b| {
            b.name("pets.update")
                .description("Update an existing pet")
        })
        .route(pets_remove, |b| {
            b.name("pets.remove")
                .description("Remove an existing pet")
        })
        .route(pets_remove, |b| {
            b.name("pets.delete")
                .description("Remove an existing pet")
                .deprecation_note("Use pets.remove instead")
        })
        .route(pets_get_first, |b| {
            b.name("pets.get-first")
                .description("Fetch first pet, if any exists")
        })
        .rename_types("reflectapi_demo::", "myapi::")
        // and some optional linting rules
        .validate(|schema| {
            let mut errors = Vec::new();
            for f in schema.functions() {
                if f.name().chars().any(|c| !c.is_ascii_alphanumeric() && c != '.' && c != '-') {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Function(f.name().to_string()),
                        "Function names must contain only alphanumeric characters, dots and dashes only".into(),
                    ));
                }
            }
            errors
        })
}

async fn health_check(
    _: Arc<AppState>,
    _request: reflectapi::Empty,
    _headers: reflectapi::Empty,
) -> reflectapi::Empty {
    ().into()
}

#[derive(Debug)]
pub struct AppState {
    pets: Mutex<Vec<model::Pet>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pets: Mutex::new(Vec::new()),
        }
    }
}

mod model {
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Pet {
        /// identity
        pub name: String,
        /// kind of pet
        pub kind: Kind,
        /// age of the pet
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub age: Option<u8>,
        /// behaviors of the pet
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub behaviors: Vec<Behavior>,
    }

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(rename_all = "snake_case")]
    pub enum Kind {
        /// A dog
        Dog,
        /// A cat
        Cat,
    }

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub enum Behavior {
        Calm,
        Aggressive(/** aggressiveness level */ f64, /** some notes */ String),
        Other {
            /// Custom provided description of a behavior
            description: String,
            /// Additional notes
            /// Up to a user to put free text here
            #[serde(default, skip_serializing_if = "String::is_empty")]
            notes: String,
        },
    }
}

fn authorize<E: From<proto::UnauthorizedError>>(headers: proto::Headers) -> Result<(), E> {
    if headers.authorization.is_empty() {
        return Err(E::from(proto::UnauthorizedError));
    }
    Ok(())
}

async fn pets_list(
    state: Arc<AppState>,
    request: proto::PetsListRequest,
    headers: proto::Headers,
) -> Result<proto::Paginated<model::Pet>, proto::PetsListError> {
    authorize::<proto::PetsListError>(headers)?;

    let pets = state.pets.lock().unwrap();
    let cursor = request
        .cursor
        .unwrap_or("0".into())
        .parse()
        .map_err(|_| proto::PetsListError::InvalidCursor)?;
    let limit = request.limit.unwrap_or(u8::MAX) as usize;
    let result_items = pets
        .iter()
        .skip(cursor)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let result_cursor = if result_items.is_empty() {
        None
    } else {
        Some((cursor + limit).to_string())
    };
    Ok(proto::Paginated {
        items: result_items,
        cursor: result_cursor,
    })
}

async fn pets_create(
    state: Arc<AppState>,
    request: proto::PetsCreateRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsCreateError> {
    authorize::<proto::PetsCreateError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    if request.0.name.is_empty() {
        return Err(proto::PetsCreateError::InvalidIdentity {
            message: "Name is required".into(),
        });
    }

    if pets.iter().any(|pet| pet.name == request.0.name) {
        return Err(proto::PetsCreateError::Conflict);
    }

    pets.push(request.0);

    Ok(().into())
}

async fn pets_update(
    state: Arc<AppState>,
    request: proto::PetsUpdateRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsUpdateError> {
    authorize::<proto::PetsUpdateError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    let Some(possition) = pets.iter().position(|pet| pet.name == request.name) else {
        return Err(proto::PetsUpdateError::NotFound);
    };
    let pet = &mut pets[possition];

    if let Some(kind) = request.kind {
        pet.kind = kind;
    }
    if let Some(age) = request.age.unfold() {
        pet.age = age.cloned();
    }
    if let Some(behaviors) = request.behaviors.unfold() {
        pet.behaviors = behaviors.cloned().unwrap_or_default();
    }

    Ok(().into())
}

async fn pets_remove(
    state: Arc<AppState>,
    request: proto::PetsRemoveRequest,
    headers: proto::Headers,
) -> Result<reflectapi::Empty, proto::PetsRemoveError> {
    authorize::<proto::PetsRemoveError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    let Some(possition) = pets.iter().position(|pet| pet.name == request.name) else {
        return Err(proto::PetsRemoveError::NotFound);
    };
    pets.remove(possition);

    Ok(().into())
}

async fn pets_get_first(
    state: Arc<AppState>,
    _: reflectapi::Empty,
    headers: proto::Headers,
) -> Result<Option<model::Pet>, proto::UnauthorizedError> {
    authorize::<proto::UnauthorizedError>(headers)?;

    let pets = state.pets.lock().unwrap();
    let random_pet = pets.first().cloned();

    Ok(random_pet)
}

mod proto {
    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct UnauthorizedError;

    impl reflectapi::StatusCode for UnauthorizedError {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::UNAUTHORIZED
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct Headers {
        pub authorization: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Paginated<T>
    where
        T: reflectapi::Output,
    {
        /// slice of a collection
        pub items: Vec<T>,
        /// cursor for getting next page
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cursor: Option<String>,
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsListRequest {
        #[serde(default)]
        pub limit: Option<u8>,
        #[serde(default)]
        pub cursor: Option<String>,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsListError {
        InvalidCursor,
        Unauthorized,
    }

    impl reflectapi::StatusCode for PetsListError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsListError::InvalidCursor => http::StatusCode::BAD_REQUEST,
                PetsListError::Unauthorized => http::StatusCode::UNAUTHORIZED,
            }
        }
    }

    impl From<UnauthorizedError> for PetsListError {
        fn from(_: UnauthorizedError) -> Self {
            PetsListError::Unauthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsCreateRequest(pub crate::model::Pet);

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsCreateError {
        Conflict,
        NotAuthorized,
        InvalidIdentity { message: String },
    }

    impl reflectapi::StatusCode for PetsCreateError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsCreateError::Conflict => http::StatusCode::CONFLICT,
                PetsCreateError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
                PetsCreateError::InvalidIdentity { .. } => http::StatusCode::UNPROCESSABLE_ENTITY,
            }
        }
    }

    impl From<UnauthorizedError> for PetsCreateError {
        fn from(_: UnauthorizedError) -> Self {
            PetsCreateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsUpdateRequest {
        /// identity
        pub name: String,
        /// kind of pet, non nullable in the model
        #[serde(default, skip_serializing_if = "Option::is_undefined")]
        pub kind: Option<crate::model::Kind>,
        /// age of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub age: reflectapi::Option<u8>,
        /// behaviors of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub behaviors: reflectapi::Option<Vec<crate::model::Behavior>>,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsUpdateError {
        NotFound,
        NotAuthorized,
    }

    impl reflectapi::StatusCode for PetsUpdateError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsUpdateError::NotFound => http::StatusCode::NOT_FOUND,
                PetsUpdateError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
            }
        }
    }

    impl From<UnauthorizedError> for PetsUpdateError {
        fn from(_: UnauthorizedError) -> Self {
            PetsUpdateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflectapi::Input)]
    pub struct PetsRemoveRequest {
        /// identity
        pub name: String,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub enum PetsRemoveError {
        NotFound,
        NotAuthorized,
    }

    impl reflectapi::StatusCode for PetsRemoveError {
        fn status_code(&self) -> http::StatusCode {
            match self {
                PetsRemoveError::NotFound => http::StatusCode::NOT_FOUND,
                PetsRemoveError::NotAuthorized => http::StatusCode::UNAUTHORIZED,
            }
        }
    }

    impl From<UnauthorizedError> for PetsRemoveError {
        fn from(_: UnauthorizedError) -> Self {
            PetsRemoveError::NotAuthorized
        }
    }
}
