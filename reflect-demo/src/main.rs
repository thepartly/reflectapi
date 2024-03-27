#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    let builder = reflect::Builder::new()
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
        });
    let (schema, handlers) = match builder.build(vec![("reflect_demo::", "myapi::")], Vec::new()) {
        Ok((schema, handlers)) => (schema, handlers),
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            return;
        }
    };

    tokio::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .await
    .unwrap();

    let app_state = Default::default();
    let axum_app = reflect::axum::into_router(app_state, handlers);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, axum_app).await.unwrap();
}

async fn health_check(
    _: std::sync::Arc<AppState>,
    _request: reflect::Empty,
    _headers: reflect::Empty,
) -> reflect::Empty {
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
    #[derive(Clone, serde::Serialize, serde::Deserialize, reflect::Input, reflect::Output)]
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

    #[derive(Clone, serde::Serialize, serde::Deserialize, reflect::Input, reflect::Output)]
    #[serde(rename_all = "snake_case", untagged)]
    pub enum Kind {
        /// A dog
        Dog,
        /// A cat
        Cat,
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize, reflect::Input, reflect::Output)]
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

pub struct UnauthorizedError;

fn authorize<E: From<UnauthorizedError>>(headers: proto::Headers) -> Result<(), E> {
    if headers.authorization.is_empty() {
        return Err(E::from(UnauthorizedError));
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
        .unwrap_or_default()
        .parse()
        .map_err(|_| proto::PetsListError::InvalidCustor)?;
    let limit = request.limit as usize;
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
) -> Result<reflect::Empty, proto::PetsCreateError> {
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
) -> Result<reflect::Empty, proto::PetsUpdateError> {
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
) -> Result<reflect::Empty, proto::PetsRemoveError> {
    authorize::<proto::PetsRemoveError>(headers)?;

    let mut pets = state.pets.lock().unwrap();

    let Some(possition) = pets.iter().position(|pet| pet.name == request.name) else {
        return Err(proto::PetsRemoveError::NotFound);
    };
    pets.remove(possition);

    Ok(().into())
}

mod proto {
    #[derive(serde::Deserialize, reflect::Input)]
    pub struct Headers {
        pub authorization: String,
    }

    #[derive(serde::Serialize, reflect::Output)]
    pub struct Paginated<T>
    where
        T: reflect::Output,
    {
        /// slice of a collection
        pub items: Vec<T>,
        /// cursor for getting next page
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cursor: Option<String>,
    }

    #[derive(serde::Deserialize, reflect::Input)]
    pub struct PetsListRequest {
        pub limit: u8,
        pub cursor: Option<String>,
    }

    #[derive(serde::Serialize, reflect::Output)]
    pub enum PetsListError {
        InvalidCustor,
        Unauthorized,
    }

    impl reflect::StatusCode for PetsListError {
        fn status_code(&self) -> u16 {
            match self {
                PetsListError::InvalidCustor => axum::http::StatusCode::BAD_REQUEST.as_u16(),
                PetsListError::Unauthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
            }
        }
    }

    impl From<super::UnauthorizedError> for PetsListError {
        fn from(_: super::UnauthorizedError) -> Self {
            PetsListError::Unauthorized
        }
    }

    #[derive(serde::Deserialize, reflect::Input)]
    pub struct PetsCreateRequest(pub crate::model::Pet);

    #[derive(serde::Serialize, reflect::Output)]
    pub enum PetsCreateError {
        Conflict,
        NotAuthorized,
        InvalidIdentity { message: String },
    }

    impl reflect::StatusCode for PetsCreateError {
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

    impl From<super::UnauthorizedError> for PetsCreateError {
        fn from(_: super::UnauthorizedError) -> Self {
            PetsCreateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflect::Input)]
    pub struct PetsUpdateRequest {
        /// identity
        pub name: String,
        /// kind of pet, non nullable in the model
        #[serde(default, skip_serializing_if = "Option::is_undefined")]
        pub kind: Option<crate::model::Kind>,
        /// age of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflect::Option::is_undefined")]
        pub age: reflect::Option<u8>,
        /// behaviors of the pet, nullable in the model
        #[serde(default, skip_serializing_if = "reflect::Option::is_undefined")]
        pub behaviors: reflect::Option<Vec<crate::model::Behavior>>,
    }

    #[derive(serde::Serialize, reflect::Output)]
    pub enum PetsUpdateError {
        NotFound,
        NotAuthorized,
    }

    impl reflect::StatusCode for PetsUpdateError {
        fn status_code(&self) -> u16 {
            match self {
                PetsUpdateError::NotFound => axum::http::StatusCode::NOT_FOUND.as_u16(),
                PetsUpdateError::NotAuthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
            }
        }
    }

    impl From<super::UnauthorizedError> for PetsUpdateError {
        fn from(_: super::UnauthorizedError) -> Self {
            PetsUpdateError::NotAuthorized
        }
    }

    #[derive(serde::Deserialize, reflect::Input)]
    pub struct PetsRemoveRequest {
        /// identity
        pub name: String,
    }

    #[derive(serde::Serialize, reflect::Output)]
    pub enum PetsRemoveError {
        NotFound,
        NotAuthorized,
    }

    impl reflect::StatusCode for PetsRemoveError {
        fn status_code(&self) -> u16 {
            match self {
                PetsRemoveError::NotFound => axum::http::StatusCode::NOT_FOUND.as_u16(),
                PetsRemoveError::NotAuthorized => axum::http::StatusCode::UNAUTHORIZED.as_u16(),
            }
        }
    }

    impl From<super::UnauthorizedError> for PetsRemoveError {
        fn from(_: super::UnauthorizedError) -> Self {
            PetsRemoveError::NotAuthorized
        }
    }
}
