//! The headers a request-headers type reads.

use reflectapi_schema::{Field, TypeReference};

use super::layout::{Layout, Lookup, Opaque, MAX_DEPTH};

/// A header a request-headers type reads.
pub(crate) struct Header {
    /// The field the header is read into; its serde name is the header's.
    pub field: Field,
    /// Every request has the header.
    pub required: bool,
}

/// The headers `headers_type` reads: the serde names of its fields and, for
/// a `#[serde(flatten)]` field, those of the struct it holds, at any depth, a
/// generic type's arguments filled in. So
/// `struct WithCommon<T> { request_id: Option<String>, #[serde(flatten)] route: T }`,
/// as `WithCommon<PetsUpdateHeaders>`, reads `request_id` and
/// `PetsUpdateHeaders`' headers. A type that isn't a struct reads none.
///
/// Fails, saying why, when a flattened field holds something other than a
/// struct or `()`, whose keys can't be headers.
pub(crate) fn request_headers(
    headers_type: &TypeReference,
    lookup: Lookup<'_, '_>,
) -> Result<Vec<Header>, String> {
    let Layout::Struct(layout) = Layout::of_type(headers_type, lookup, 0) else {
        return Ok(Vec::new());
    };
    match layout.keyed_fields() {
        Ok(fields) => Ok(fields
            .into_iter()
            .map(|keyed| Header {
                field: keyed.field.clone(),
                required: keyed.required,
            })
            .collect()),
        Err(group) => Err(match group.layout {
            Layout::Unknown(Opaque::TooDeep) => {
                format!("its flattened fields nest deeper than {MAX_DEPTH}")
            }
            _ => format!(
                "flattened field `{}` holds a type other than a struct with named fields",
                group.field.name
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflectapi_schema::{Enum, Fields, Representation, Struct, Type};

    fn strukt(name: &str, fields: Vec<Field>) -> Type {
        let mut strukt = Struct::new(name);
        strukt.fields = Fields::Named(fields);
        Type::Struct(strukt)
    }

    fn headers(type_name: &str, types: &[Type]) -> Result<Vec<(String, bool)>, String> {
        let lookup = |name: &str| types.iter().find(|ty| ty.name() == name);
        Ok(request_headers(&type_name.into(), &lookup)?
            .into_iter()
            .map(|header| (header.field.serde_name().to_owned(), header.required))
            .collect())
    }

    /// The shape of `PetsUpdateHeaders` in the docs' Request Headers.
    #[test]
    fn a_flattened_struct_gives_its_fields_as_headers() {
        let mut request_id = Field::new("request_id".into(), "std::string::String".into());
        request_id.serde_name = "x-request-id".into();
        let mut common = Field::new("common".into(), "CommonHeaders".into());
        common.flattened = true;
        let types = [
            strukt(
                "PetsUpdateHeaders",
                vec![
                    Field::new("authorization".into(), "std::string::String".into())
                        .with_required(true),
                    common,
                ],
            ),
            strukt("CommonHeaders", vec![request_id]),
        ];
        assert_eq!(
            headers("PetsUpdateHeaders", &types).unwrap(),
            [
                ("authorization".to_owned(), true),
                ("x-request-id".to_owned(), false)
            ]
        );
    }

    #[test]
    fn a_type_that_is_not_a_struct_reads_no_headers() {
        let types = [Type::Enum(Enum::new("Headers".into()))];
        assert_eq!(headers("Headers", &types).unwrap(), []);
        assert_eq!(headers("Missing", &types).unwrap(), []);
    }

    #[test]
    fn a_flattened_enum_has_no_headers_to_read() {
        let mut auth = Field::new("auth".into(), "Auth".into());
        auth.flattened = true;
        let mut enm = Enum::new("Auth".into());
        enm.representation = Representation::Internal { tag: "kind".into() };
        let types = [strukt("Headers", vec![auth]), Type::Enum(enm)];
        assert_eq!(
            headers("Headers", &types).unwrap_err(),
            "flattened field `auth` holds a type other than a struct with named fields"
        );
    }
}
