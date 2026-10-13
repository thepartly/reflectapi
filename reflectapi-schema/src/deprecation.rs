use core::fmt;

/// Deprecation metadata of a function, field or variant, borrowed from it.
///
/// Returned by `deprecation()` on [`crate::Function`], [`crate::Field`] and
/// [`crate::Variant`] when the item is deprecated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deprecation<'a> {
    /// Free-form note, often naming the replacement. May be empty.
    pub note: &'a str,
    /// Date the item was deprecated, `YYYY-MM-DD`.
    pub since: Option<&'a str>,
    /// Date after which the item may be removed, `YYYY-MM-DD`.
    pub sunset: Option<&'a str>,
}

impl<'a> Deprecation<'a> {
    pub(crate) fn of(
        note: &'a Option<String>,
        since: &'a Option<String>,
        sunset: &'a Option<String>,
    ) -> Option<Self> {
        // Dates without a note still mark the item deprecated, so a schema
        // built by hand can't end up with dates on an item that isn't.
        if note.is_none() && since.is_none() && sunset.is_none() {
            return None;
        }
        Some(Deprecation {
            note: note.as_deref().unwrap_or_default(),
            since: since.as_deref(),
            sunset: sunset.as_deref(),
        })
    }

    /// Human-readable message combining the dates and the note, for example
    /// `Deprecated since 2026-10-01; removal after 2026-11-01. Use X instead.`
    ///
    /// It is the note unchanged when there are no dates, and empty when there
    /// is neither a note nor a date.
    pub fn message(&self) -> String {
        let dates = match (self.since, self.sunset) {
            (Some(since), Some(sunset)) => {
                format!("Deprecated since {since}; removal after {sunset}.")
            }
            (Some(since), None) => format!("Deprecated since {since}."),
            (None, Some(sunset)) => format!("Removal after {sunset}."),
            (None, None) => String::new(),
        };
        match (dates.is_empty(), self.note.is_empty()) {
            (true, _) => self.note.to_owned(),
            (false, true) => dates,
            (false, false) => format!("{dates} {}", self.note),
        }
    }
}

/// Why a deprecation date was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidDeprecationDate {
    /// Not a calendar date in `YYYY-MM-DD` form.
    Format { value: String },
    /// The sunset date is before the date the item was deprecated.
    SunsetBeforeSince { since: String, sunset: String },
}

impl fmt::Display for InvalidDeprecationDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format { value } => {
                write!(
                    f,
                    "invalid date `{value}`, expected a calendar date as YYYY-MM-DD"
                )
            }
            Self::SunsetBeforeSince { since, sunset } => write!(
                f,
                "sunset date {sunset} is before the deprecation date {since}"
            ),
        }
    }
}

impl std::error::Error for InvalidDeprecationDate {}

/// Checks that `since` and `sunset`, when present, are calendar dates in
/// `YYYY-MM-DD` form and that `sunset` is not before `since`.
pub fn validate_deprecation_dates(
    since: Option<&str>,
    sunset: Option<&str>,
) -> Result<(), InvalidDeprecationDate> {
    for date in since.into_iter().chain(sunset) {
        if !is_iso_date(date) {
            return Err(InvalidDeprecationDate::Format {
                value: date.to_owned(),
            });
        }
    }
    if let (Some(since), Some(sunset)) = (since, sunset) {
        // Validated `YYYY-MM-DD` strings order chronologically.
        if sunset < since {
            return Err(InvalidDeprecationDate::SunsetBeforeSince {
                since: since.to_owned(),
                sunset: sunset.to_owned(),
            });
        }
    }
    Ok(())
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |range: core::ops::Range<usize>| -> Option<u32> {
        b[range].iter().try_fold(0u32, |acc, c| {
            c.is_ascii_digit().then(|| acc * 10 + u32::from(c - b'0'))
        })
    };
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_calendar_dates() {
        for date in ["2026-10-01", "2024-02-29", "2000-02-29", "1999-12-31"] {
            assert!(is_iso_date(date), "{date}");
        }
    }

    #[test]
    fn rejects_malformed_or_impossible_dates() {
        for date in [
            "",
            "2026-1-01",
            "2026/10/01",
            "26-10-01",
            "2026-13-01",
            "2026-00-10",
            "2026-04-31",
            "2026-02-29",
            "1900-02-29",
            "2026-10-01T00:00:00Z",
            "２０２６-10-01",
            "+026-10-01",
        ] {
            assert!(!is_iso_date(date), "{date}");
        }
    }

    #[test]
    fn rejects_sunset_before_since() {
        assert_eq!(
            validate_deprecation_dates(Some("2026-10-01"), Some("2026-09-30")),
            Err(InvalidDeprecationDate::SunsetBeforeSince {
                since: "2026-10-01".into(),
                sunset: "2026-09-30".into()
            })
        );
        assert!(validate_deprecation_dates(Some("2026-10-01"), Some("2026-10-01")).is_ok());
    }

    #[test]
    fn composes_messages() {
        let message = |note: &str, since, sunset| {
            Deprecation {
                note,
                since,
                sunset,
            }
            .message()
        };
        assert_eq!(message("", None, None), "");
        assert_eq!(message("Use X instead.", None, None), "Use X instead.");
        assert_eq!(
            message("Use X instead.", Some("2026-10-01"), Some("2026-11-01")),
            "Deprecated since 2026-10-01; removal after 2026-11-01. Use X instead."
        );
        assert_eq!(
            message("", Some("2026-10-01"), None),
            "Deprecated since 2026-10-01."
        );
        assert_eq!(
            message("Use X instead.", None, Some("2026-11-01")),
            "Removal after 2026-11-01. Use X instead."
        );
    }
}
