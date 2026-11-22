//! Includes every page of the book as the doc comment of a module, so that
//! `cargo test --doc` compiles and runs its Rust code blocks.

use std::{env, fs, path::Path};

fn main() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    println!("cargo:rerun-if-changed={}", src.display());

    let mut pages = Vec::new();
    collect(&src, &mut pages);
    pages.sort();

    let mut out = String::new();
    for page in &pages {
        println!("cargo:rerun-if-changed={}", page.display());
        let name = page
            .strip_prefix(&src)
            .unwrap()
            .with_extension("")
            .to_string_lossy()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
            .to_lowercase();
        out += &format!(
            "#[doc = include_str!({:?})]\npub mod {name} {{}}\n",
            page.canonicalize().unwrap()
        );
    }
    let out_path = Path::new(&env::var("OUT_DIR").unwrap()).join("pages.rs");
    fs::write(out_path, out).unwrap();
}

fn collect(dir: &Path, pages: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, pages);
        } else if path.extension().is_some_and(|e| e == "md") {
            pages.push(path);
        }
    }
}
