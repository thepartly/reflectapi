mod format;
pub mod rust;
pub mod typescript;

use self::format::format_with;

#[derive(Debug, Default)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    pub format: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    pub typecheck: bool,
    pub shared_modules: Vec<String>,
}
