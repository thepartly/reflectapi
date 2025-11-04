use std::ops::ControlFlow;

use crate::Visitor;

#[cfg(feature = "glob")]
pub use self::glob::Glob;

pub(crate) struct Renamer<'a, P> {
    pattern: P,
    replacer: &'a str,
}

impl<'a, P: Pattern> Renamer<'a, P> {
    pub fn new(pattern: P, replacer: &'a str) -> Self {
        Self { pattern, replacer }
    }
}

impl<P: Pattern> Visitor for Renamer<'_, P> {
    type Output = usize;

    fn visit_top_level_name(
        &mut self,
        name: &mut String,
    ) -> ControlFlow<Self::Output, Self::Output> {
        ControlFlow::Continue(
            self.pattern
                .rename(name, self.replacer)
                .map_or(0, |new_name| {
                    *name = new_name;
                    1
                }),
        )
    }
}

#[allow(private_bounds)]
pub trait Pattern: Sealed + Copy {
    fn rename(self, name: &str, with: &str) -> Option<String>;
}

impl Sealed for &str {}

impl Pattern for &str {
    fn rename(self, name: &str, replacer: &str) -> Option<String> {
        if self.ends_with("::") {
            // replacing module name
            if let Some(name) = name.strip_prefix(self) {
                let replacer = replacer.trim_end_matches("::");
                if replacer.is_empty() {
                    return Some(name.into());
                }

                Some(format!("{replacer}::{name}"))
            } else {
                None
            }
        } else {
            // replacing type name
            if name == self {
                Some(replacer.into())
            } else {
                None
            }
        }
    }
}

#[cfg(feature = "glob")]
mod glob {
    use core::fmt;
    use std::str::FromStr;

    use super::{Pattern, Sealed};

    /// Bulk renamer of modules using glob patterns.
    #[derive(Clone)]
    pub struct Glob(glob::Pattern);

    impl fmt::Display for Glob {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0.to_string().replace('/', "::"))
        }
    }

    impl fmt::Debug for Glob {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{self}")
        }
    }

    impl FromStr for Glob {
        type Err = glob::PatternError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let s = s.replace("::", "/");
            Ok(Self(glob::Pattern::new(&s)?))
        }
    }

    impl Sealed for &Glob {}

    impl Pattern for &Glob {
        fn rename(self, name: &str, replacer: &str) -> Option<String> {
            let path = std::path::PathBuf::from(name.replace("::", "/"));
            let file_name = path.file_name().expect("non-empty name");

            if path.components().count() <= 1 {
                // This means the `name` has no module, so we don't replace it.
                return None;
            }

            if self.0.matches_path_with(
                &path,
                glob::MatchOptions {
                    case_sensitive: true,
                    require_literal_separator: true,
                    require_literal_leading_dot: false,
                },
            ) {
                let replacer = replacer.trim_end_matches("::");
                let name = file_name
                    .to_str()
                    .expect("is utf-8 as it was converted from str");
                if replacer.is_empty() {
                    return Some(name.into());
                }

                Some(format!("{replacer}::{name}"))
            } else {
                None
            }
        }
    }
}

trait Sealed {}

#[cfg(test)]
mod tests {
    use super::Pattern;

    #[track_caller]
    fn check(matcher: impl Pattern, name: &str, replacer: &str, expected: Option<&str>) {
        assert_eq!(matcher.rename(name, replacer).as_deref(), expected);
    }

    // Confusing to read if calling function directly
    macro_rules! rename {
        ($matcher:literal to $replacer:literal on $name:expr => $expected:expr) => {
            check($matcher, $name, $replacer, $expected);
        };
    }

    macro_rules! glob_rename {
        ($matcher:literal to $replacer:literal on $name:expr => $expected:expr) => {
            #[cfg(feature = "glob")]
            check(
                &$matcher.parse::<super::Glob>().unwrap(),
                $name,
                $replacer,
                $expected,
            );
        };
    }

    #[test]
    fn rename_str() {
        rename!("foo" to "bar" on "foo" => Some("bar"));
        rename!("foo" to "bar" on "baz" => None);

        rename!("foo::" to "" on "foo::bar" => Some("bar"));

        rename!("foo::" to "bar::" on "foo::a" => Some("bar::a"));
        // Optional trailing `::` on replacer
        rename!("foo::" to "bar" on "foo::a" => Some("bar::a"));
    }

    #[test]
    fn rename_glob() {
        // globs should only match modules
        glob_rename!("foo*" to "bar" on "foo" => None);
        glob_rename!("foo::*" to "bar::" on "foo::a" => Some("bar::a"));
        // Optional trailing `::` on replacer
        glob_rename!("foo::*" to "bar" on "foo::a" => Some("bar::a"));
        glob_rename!("foo::*" to "" on "foo::a" => Some("a"));
        glob_rename!("foo::*" to "bar" on "foo::a" => Some("bar::a"));
        glob_rename!("foo::**" to "bar" on "foo::a::b::c" => Some("bar::c"));

        // Must be a full match
        glob_rename!("fo*" to "bar" on "foo::a" => None);
    }
}
