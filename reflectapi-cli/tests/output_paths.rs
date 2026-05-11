//! Integration tests for `reflectapi codegen --output …` path handling.
//!
//! Multi-file codegen (TS, Python) needs to handle:
//!   - existing directories
//!   - fresh directories (path doesn't exist yet)
//!   - file-shaped paths whose filename matches one of the emitted files
//!     (siblings land in the parent directory)
//!   - stdout via `--output -`, which must print the language's *primary*
//!     file rather than the alphabetically-first one.

use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    let bin = env!("CARGO_BIN_EXE_reflectapi");
    std::path::PathBuf::from(bin)
}

fn demo_schema() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("reflectapi-demo")
        .join("reflectapi.json")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .output()
        .expect("spawn reflectapi")
}

#[test]
fn ts_output_into_fresh_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("brand-new-dir");
    let schema = demo_schema();
    let out = run(&[
        "codegen",
        "--language",
        "typescript",
        "--schema",
        schema.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "exit={:?}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(target.is_dir(), "expected fresh dir to be created");
    assert!(target.join("generated.ts").is_file());
    assert!(target.join("generated.transport.ts").is_file());
}

#[test]
fn python_output_into_fresh_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("python-client");
    let schema = demo_schema();
    let out = run(&[
        "codegen",
        "--language",
        "python",
        "--schema",
        schema.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "exit={:?}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(target.is_dir());
    assert!(target.join("generated.py").is_file());
    assert!(target.join("__init__.py").is_file());
}

#[test]
fn ts_output_to_file_path_writes_siblings() {
    // --output …/generated.ts should still work; transport file lands
    // alongside it in the parent directory.
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("generated.ts");
    let schema = demo_schema();
    let out = run(&[
        "codegen",
        "--language",
        "typescript",
        "--schema",
        schema.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(target.is_file(), "primary file at requested path");
    assert!(tmp.path().join("generated.transport.ts").is_file());
}

#[test]
fn ts_stdout_emits_primary_file_not_transport() {
    // --output - should print generated.ts (the API surface), not
    // generated.transport.ts (the helper). BTreeMap ordering would
    // pick the latter alphabetically.
    let schema = demo_schema();
    let out = run(&[
        "codegen",
        "--language",
        "typescript",
        "--schema",
        schema.to_str().unwrap(),
        "--output",
        "-",
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // generated.ts contains the public `client` factory; the
    // transport file does not.
    assert!(
        stdout.contains("export function client(") || stdout.contains("export function client "),
        "expected stdout to be generated.ts (with the `client` factory). got:\n{stdout}",
    );
}

#[test]
fn python_stdout_emits_generated_not_init() {
    let schema = demo_schema();
    let out = run(&[
        "codegen",
        "--language",
        "python",
        "--schema",
        schema.to_str().unwrap(),
        "--output",
        "-",
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // __init__.py is a package-root shim; generated.py is the flat
    // compatibility facade and should include model re-exports.
    assert!(
        stdout.contains("from ._rebuild import rebuild_models")
            && stdout.contains("from .myapi.proto import"),
        "expected stdout to be generated.py compatibility facade. got:\n{stdout}",
    );
}
