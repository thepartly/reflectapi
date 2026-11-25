//! Cursor pagination. A cursor is a `String` by default; a server that wants
//! its contents typed uses its own cursor type that is a string on the wire,
//! such as `Encoded<Cursor>`, which then decodes on arrival, and clients still
//! see a string.

use std::num::NonZeroU32;

/// The page a list request asks for.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Input)]
pub struct PageRequest<C = String> {
    /// Where to start: a `next_cursor` from an earlier response, or none for
    /// the first page.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub cursor: Option<C>,
    /// How many items to return, at most. The server may return fewer, even
    /// when more follow: only an absent `next_cursor` means the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<NonZeroU32>,
}

/// A list returned whole. Its items are under `items`, as in a paginated
/// list, so pagination can be added later without breaking clients.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Output)]
pub struct NonPaginatedList<D> {
    pub items: Vec<D>,
}

/// A page of a list, read forward with [`PageRequest::cursor`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Output)]
pub struct ForwardPaginatedList<D, C = String> {
    pub items: Vec<D>,
    pub pagination: Option<ForwardPagination<C>>,
}

/// Where the next page of a [`ForwardPaginatedList`] starts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, reflectapi::Output)]
pub struct ForwardPagination<C = String> {
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<C>,
}

// By hand: a derive would require `C: Default`, which a typed cursor needn't
// be.
impl<C> Default for PageRequest<C> {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: None,
        }
    }
}

impl<C> Default for ForwardPagination<C> {
    fn default() -> Self {
        Self { next_cursor: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_request_may_be_empty() {
        let request: PageRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(request, PageRequest::default());
        assert_eq!(serde_json::to_string(&request).unwrap(), "{}");
        assert!(serde_json::from_str::<PageRequest>(r#"{"limit":0}"#).is_err());
    }

    #[test]
    fn a_typed_cursor_decodes_on_arrival() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Cursor {
            after_id: u64,
        }
        let cursor = reflectapi::Encoded(Cursor { after_id: 42 })
            .encode()
            .unwrap();
        let request: PageRequest<reflectapi::Encoded<Cursor>> =
            serde_json::from_str(&format!(r#"{{"cursor":"{cursor}"}}"#)).unwrap();
        assert_eq!(request.cursor.unwrap().0, Cursor { after_id: 42 });
        assert!(
            serde_json::from_str::<PageRequest<reflectapi::Encoded<Cursor>>>(
                r#"{"cursor":"not a cursor"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn a_list_keeps_its_items_under_items() {
        let list = ForwardPaginatedList {
            items: vec![1, 2],
            pagination: Some(ForwardPagination {
                next_cursor: Some("c2".to_owned()),
            }),
        };
        assert_eq!(
            serde_json::to_string(&list).unwrap(),
            r#"{"items":[1,2],"pagination":{"next_cursor":"c2"}}"#
        );
    }
}
