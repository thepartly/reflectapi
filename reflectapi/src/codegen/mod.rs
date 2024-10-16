mod format;
pub mod openapi;
pub mod rust;
pub mod typescript;

use std::{
    hash::{DefaultHasher, Hasher},
    path::PathBuf,
};

use self::format::format_with;

#[derive(Debug, Default)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    pub format: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    pub typecheck: bool,
    pub shared_modules: Vec<String>,
    /// Only include handlers with these tags (empty means include all).
    pub include_tags: Vec<String>,
    /// Exclude handlers with these tags (empty means exclude none).
    pub exclude_tags: Vec<String>,
}

fn tmp_path(src: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    hasher.write(src.as_bytes());
    let hash = hasher.finish();

    std::env::temp_dir().join(format!("reflectapi-{hash}"))
}

// Comes in pairs, anything between them is boilerplate and can be removed for tests snapshots.
const START_BOILERPLATE: &str = "/* <----- */";
const END_BOILERPLATE: &str = "/* -----> */";

#[doc(hidden)]
pub fn strip_boilerplate(src: &str) -> String {
    let mut stripped = String::new();
    let mut skip = false;
    for line in src.lines() {
        if line.contains(START_BOILERPLATE) {
            assert!(!skip, "nested start boilerplate markers");
            skip = true;
            continue;
        }

        if line.contains(END_BOILERPLATE) {
            assert!(skip, "unmatched end boilerplate marker");
            skip = false;
            continue;
        }

        if !skip {
            stripped.push_str(line);
            stripped.push('\n');
        }
    }

    if skip {
        panic!("unmatched boilerplate marker");
    }

    stripped
}
