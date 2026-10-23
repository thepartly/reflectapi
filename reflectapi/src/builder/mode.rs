use http::HeaderName;

use crate::{Function, Input, Output, OutputType, Schema, Struct, TypeReference, Typespace};

use super::{BuildError, OutputTypeFn, RouteBuilder};

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::WithSchema {}
    impl Sealed for super::DispatchOnly {}
}

/// What a [`crate::Builder`] produces: the schema and the routes, or the routes alone.
///
/// Write the API definition once, generic over the mode, and build it in the
/// mode each consumer needs:
///
/// ```rust,ignore
/// pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<AppState, M> {
///     reflectapi::Builder::default().route(handler, |b| b.name("pets.list"))
/// }
///
/// // Build time (codegen, CI): the schema, plus the routes.
/// let (schema, _) = api::<reflectapi::WithSchema>().build()?;
/// // Serving: the routes alone; no type is reflected.
/// let routers = api::<reflectapi::DispatchOnly>().build()?;
/// ```
pub trait BuildMode: sealed::Sealed + Send + 'static {
    /// Whether this mode builds the schema; schema-only builder steps are skipped otherwise.
    #[doc(hidden)]
    const WITH_SCHEMA: bool;

    /// Records the function in `schema` (when building it) and returns the
    /// names of the request headers the handler reads.
    #[doc(hidden)]
    fn register_function<I: Input, H: Input, O: Output, E: Output>(
        route: &RouteBuilder,
        schema: &mut Schema,
        is_stream: bool,
    ) -> Result<Vec<HeaderName>, BuildError>;

    /// `H::reflectapi_output_type` when building the schema, `None` otherwise.
    #[doc(hidden)]
    fn response_headers_type<H: Output>() -> Option<OutputTypeFn>;
}

/// Builds the schema together with the routes. The default mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct WithSchema;

/// Builds the routes only, for serving: types are not reflected, so the
/// schema is never built at runtime and its code is not linked into the
/// binary. Produce the schema at build time with [`WithSchema`].
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchOnly;

impl BuildMode for WithSchema {
    const WITH_SCHEMA: bool = true;

    fn register_function<I: Input, H: Input, O: Output, E: Output>(
        route: &RouteBuilder,
        schema: &mut Schema,
        is_stream: bool,
    ) -> Result<Vec<HeaderName>, BuildError> {
        let input_type = I::reflectapi_input_type(&mut schema.input_types);
        let output_type = O::reflectapi_output_type(&mut schema.output_types);
        let error_type = E::reflectapi_output_type(&mut schema.output_types);
        let input_headers = H::reflectapi_input_type(&mut schema.input_types);
        let input_header_names = header_names(&schema.input_types, &input_headers)?;

        schema.functions.push(Function {
            name: route.name.clone(),
            path: route.path.clone(),
            deprecation_note: route.deprecation_note.clone(),
            deprecated_in: route.deprecated_in.clone(),
            deprecated_since: route.deprecated_since.clone(),
            sunset: route.sunset.clone(),
            description: route.description.clone(),
            input_type: if input_type.name == "reflectapi::Empty" {
                None
            } else {
                Some(input_type)
            },
            output_type: if is_stream {
                OutputType::Stream {
                    item_type: output_type,
                }
            } else {
                OutputType::Complete {
                    output_type: if output_type.name == "reflectapi::Empty" {
                        None
                    } else {
                        Some(output_type)
                    },
                }
            },
            error_type: if error_type.name == "reflectapi::Infallible" {
                None
            } else {
                Some(error_type)
            },
            response_headers: route
                .response_headers
                .map(|reflect_output_type| reflect_output_type(&mut schema.output_types)),
            input_headers: if input_header_names.is_empty() {
                None
            } else {
                Some(input_headers)
            },
            serialization: vec![
                crate::SerializationMode::Json,
                #[cfg(feature = "msgpack")]
                crate::SerializationMode::Msgpack,
            ],
            readonly: route.readonly,
            tags: route.tags.clone(),
        });

        Ok(input_header_names)
    }

    fn response_headers_type<H: Output>() -> Option<OutputTypeFn> {
        Some(H::reflectapi_output_type)
    }
}

impl BuildMode for DispatchOnly {
    const WITH_SCHEMA: bool = false;

    fn register_function<I: Input, H: Input, O: Output, E: Output>(
        _route: &RouteBuilder,
        _schema: &mut Schema,
        _is_stream: bool,
    ) -> Result<Vec<HeaderName>, BuildError> {
        // Only the headers type is reflected, into a scratch typespace, for
        // its field names: the handler forwards just those headers.
        let mut typespace = Typespace::new();
        let input_headers = H::reflectapi_input_type(&mut typespace);
        header_names(&typespace, &input_headers)
    }

    fn response_headers_type<H: Output>() -> Option<OutputTypeFn> {
        None
    }
}

/// The header names a request-headers type reads: the serde names of its fields.
fn header_names(
    typespace: &Typespace,
    headers_type: &TypeReference,
) -> Result<Vec<HeaderName>, BuildError> {
    let Some(crate::Type::Struct(Struct { fields, .. })) =
        typespace.get_type(headers_type.name.as_str())
    else {
        return Ok(Vec::new());
    };
    fields
        .iter()
        .map(|field| {
            let header_name = if field.serde_name.is_empty() {
                field.name.as_str()
            } else {
                field.serde_name.as_str()
            };
            HeaderName::from_bytes(header_name.as_bytes()).map_err(|_| {
                BuildError::Other(
                    format!(
                        "request headers type `{}`: `{header_name}` is not a valid header name",
                        headers_type.name
                    )
                    .into(),
                )
            })
        })
        .collect()
}
