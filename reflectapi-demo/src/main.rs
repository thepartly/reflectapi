#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    // build an application, by providing pointers to handlers
    // and assigning those to named routes
    let builder = reflectapi::Builder::new()
        .name("Demo application".to_string())
        .description("This is a demo application".to_string())
        .route(health_check, |b| {
            b.name("health.check".into())
                .readonly(true)
                .description("Check the health of the service".into())
        })
        .route(pets_list, |b| {
            b.name("pets.list".into())
                .readonly(true)
                .description("List available pets".into())
        })
        .route(pets_create, |b| {
            b.name("pets.create".into())
                .description("Create a new pet".into())
        })
        .route(pets_update, |b| {
            b.name("pets.update".into())
                .description("Update an existing pet".into())
        })
        .route(pets_remove, |b| {
            b.name("pets.remove".into())
                .description("Remove an existing pet".into())
        })
        .route(pets_get_first, |b| {
            b.name("pets.get-first".into())
                .description("Fetch first pet, if any exists".into())
        })
        // some optional tuning
        .fold_transparent_types()
        .rename_type("reflectapi_demo::", "myapi::")
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
        });
    let (schema, routers) = match builder.build() {
        Ok((schema, routers)) => (schema, routers),
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        }
    };

    // write reflect schema to a file
    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    // start the server based on axum web framework
    let app_state = Default::default();
    let axum_app = reflectapi::axum::into_router(app_state, routers, |_name, r| {
        // let's append some tracing middleware
        // it can be different depending on the router name,
        // (we have only 1 in the demo example)
        r.layer(tower_http::trace::TraceLayer::new_for_http())
    });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}

async fn health_check(
    _: std::sync::Arc<AppState>,
    _request: reflectapi::Empty,
    _headers: reflectapi::Empty,
) -> reflectapi::Empty {
    ().into()
}

struct AppState {
    pets: std::sync::Mutex<Vec<model::Pet>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pets: std::sync::Mutex::new(Vec::new()),
        }
    }
}

mod model {
    #[derive(
        Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
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
        Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    #[serde(rename_all = "snake_case")]
    pub enum Kind {
        /// A dog
        Dog,
        /// A cat
        Cat,
    }

    #[derive(
        Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
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
    state: std::sync::Arc<AppState>,
    request: proto::PetsListRequest,
    headers: proto::Headers,
) -> Result<proto::Paginated<model::Pet>, proto::PetsListError> {
    authorize::<proto::PetsListError>(headers)?;

    let pets = state.pets.lock().unwrap();
    let cursor = request
        .cursor
        .unwrap_or("0".into())
        .parse()
        .map_err(|_| proto::PetsListError::InvalidCustor)?;
    let limit = request.limit.unwrap_or(u8::MAX) as usize;
    let result_items = pets
        .iter()
        .skip(cursor)
        .take(limit)
        .map(|i| i.clone())
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
    state: std::sync::Arc<AppState>,
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
    state: std::sync::Arc<AppState>,
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
    state: std::sync::Arc<AppState>,
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
    state: std::sync::Arc<AppState>,
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
        fn status_code(&self) -> u16 {
            axum::http::StatusCode::UNAUTHORIZED.as_u16()
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
        InvalidCustor,
        Unauthorized,
    }

    impl reflectapi::StatusCode for PetsListError {
        fn status_code(&self) -> u16 {
            match self {
                PetsListError::InvalidCustor => axum::http::StatusCode::BAD_REQUEST.as_u16(),
                PetsListError::Unauthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
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
        fn status_code(&self) -> u16 {
            match self {
                PetsCreateError::Conflict => axum::http::StatusCode::CONFLICT.as_u16(),
                PetsCreateError::NotAuthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
                PetsCreateError::InvalidIdentity { .. } => {
                    axum::http::StatusCode::UNPROCESSABLE_ENTITY.as_u16()
                }
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
        fn status_code(&self) -> u16 {
            match self {
                PetsUpdateError::NotFound => axum::http::StatusCode::NOT_FOUND.as_u16(),
                PetsUpdateError::NotAuthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
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
        fn status_code(&self) -> u16 {
            match self {
                PetsRemoveError::NotFound => axum::http::StatusCode::NOT_FOUND.as_u16(),
                PetsRemoveError::NotAuthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
            }
        }
    }

    impl From<UnauthorizedError> for PetsRemoveError {
        fn from(_: UnauthorizedError) -> Self {
            PetsRemoveError::NotAuthorized
        }
    }
}
