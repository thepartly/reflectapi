use core::fmt;

use serde::ser::Serialize;

use crate::Output;

/// One event of a stream route: an item with the Server-Sent Events `id` to
/// send with it.
///
/// A stream route yields its items either as they are, sent without an id, or
/// wrapped in `Event` to give each one an id. The id is the standard SSE
/// `id` field: a client, or a server or proxy that resumes the stream for
/// it, sends the last one back in the `Last-Event-ID` request header, and the
/// route can declare that header to continue after it. The schema, and so
/// every generated client, sees only the item: wrapping items in `Event`
/// does not change the API.
///
/// ```rust,ignore
/// fn pets_events(
///     state: AppState,
///     _: reflectapi::Empty,
///     headers: Headers,
/// ) -> Result<impl Stream<Item = reflectapi::Event<PetEvent>>, Error> {
///     let events = state.events_after(headers.last_event_id.as_deref());
///     Ok(events.map(|event: PetEvent| reflectapi::Event::new(event.seq, event)))
/// }
/// ```
///
/// An integer or a `uuid::Uuid` converts to an [`EventId`] as is; a string
/// is checked by [`EventId::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<T> {
    pub id: Option<EventId>,
    pub data: T,
}

impl<T> Event<T> {
    /// An event with `data` and the id `id`.
    pub fn new(id: impl Into<EventId>, data: T) -> Self {
        Self {
            id: Some(id.into()),
            data,
        }
    }
}

/// The id of an [`Event`]: a string with no line break or NUL, which could
/// not be sent as the SSE `id` field. An empty id clears the client's last
/// event id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(String);

impl EventId {
    /// `id`, or an error if it contains a line break or NUL.
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidEventId> {
        let id = id.into();
        if id.contains(['\n', '\r', '\0']) {
            return Err(InvalidEventId(id));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for EventId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for EventId {
    type Error = InvalidEventId;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

impl TryFrom<&str> for EventId {
    type Error = InvalidEventId;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

macro_rules! event_id_from_display {
    ($($t:ty),*) => {$(
        impl From<$t> for EventId {
            fn from(id: $t) -> Self {
                Self(id.to_string())
            }
        }
    )*};
}

event_id_from_display!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

#[cfg(feature = "uuid")]
event_id_from_display!(uuid::Uuid);

/// A string given as an [`EventId`] that contains a line break or NUL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEventId(String);

impl fmt::Display for InvalidEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event id contains a line break or NUL: {:?}", self.0)
    }
}

impl std::error::Error for InvalidEventId {}

/// An item of a stream route's stream: an [`Output`] type, sent without an
/// event id, or an [`Event`] of one. Implemented for those only.
pub trait StreamItem: sealed::Sealed {
    /// The item as the schema shows it.
    type Data: Output + Serialize;

    fn into_event(self) -> Event<Self::Data>;
}

impl<T: Output + Serialize> StreamItem for T {
    type Data = T;

    fn into_event(self) -> Event<T> {
        Event {
            id: None,
            data: self,
        }
    }
}

impl<T: Output + Serialize> StreamItem for Event<T> {
    type Data = T;

    fn into_event(self) -> Event<T> {
        self
    }
}

mod sealed {
    pub trait Sealed {}

    impl<T: crate::Output + serde::ser::Serialize> Sealed for T {}

    impl<T: crate::Output + serde::ser::Serialize> Sealed for super::Event<T> {}
}

#[cfg(test)]
mod tests {
    use super::EventId;

    #[test]
    fn event_id_rejects_line_breaks_and_nul() {
        for id in ["a\nb", "a\rb", "a\0b", "\n"] {
            assert_eq!(
                EventId::new(id).map_err(|err| err.to_string()),
                Err(format!("event id contains a line break or NUL: {id:?}"))
            );
        }
        assert_eq!(
            EventId::new("").map(EventId::into_string),
            Ok(String::new())
        );
        assert_eq!(EventId::from(42u32).as_str(), "42");
    }
}
