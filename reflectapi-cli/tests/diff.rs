use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

fn demo_schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../reflectapi-demo/reflectapi.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The demo schema, and one where `Kind` gains `fish` and `pets.delete` is gone.
fn schemas(dir: &Path) -> (PathBuf, PathBuf) {
    let old = demo_schema();
    let mut new = old.clone();
    for types in ["input_types", "output_types"] {
        for ty in new[types]["types"].as_array_mut().unwrap() {
            if ty["name"] == "myapi::model::Kind" {
                ty["variants"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({ "name": "fish", "fields": "none" }));
            }
        }
    }
    new["functions"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "pets.delete");

    let (old_path, new_path) = (dir.join("old.json"), dir.join("new.json"));
    std::fs::write(&old_path, old.to_string()).unwrap();
    std::fs::write(&new_path, new.to_string()).unwrap();
    (old_path, new_path)
}

fn diff(dir: &Path, args: &[&str]) -> Output {
    let (old, new) = schemas(dir);
    Command::new(env!("CARGO_BIN_EXE_reflectapi"))
        .arg("diff")
        .arg(old)
        .arg(new)
        .args(["--today", "2026-11-20"])
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn breaking_changes_exit_1_and_suggest_acknowledgements() {
    let dir = tempfile::tempdir().unwrap();
    let output = diff(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(
        text.contains("pets.delete: function removed, deprecated without a sunset"),
        "{text}"
    );
    assert!(
        text.contains("- pets.list output body.items[].kind::fish [variant-added]: <reason, consumers checked, ticket>"),
        "{text}"
    );
}

#[test]
fn warned_and_acknowledged_breaks_exit_0() {
    let dir = tempfile::tempdir().unwrap();
    let acks = dir.path().join("acks.md");
    std::fs::write(
        &acks,
        "- pets.delete [function-removed]: replaced by pets.remove, PETS-1\n",
    )
    .unwrap();

    let output = diff(
        dir.path(),
        &[
            "--warn",
            "variant-added",
            "--acknowledged",
            acks.to_str().unwrap(),
        ],
    );
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("3 warning(s):"), "{text}");
    assert!(
        text.contains("1 acknowledged breaking change(s):"),
        "{text}"
    );
    assert!(
        text.contains("no unacknowledged breaking changes"),
        "{text}"
    );
}

#[test]
fn json_lists_severity_kind_and_key() {
    let dir = tempfile::tempdir().unwrap();
    let output = diff(dir.path(), &["--json", "--warn", "variant-added"]);
    assert_eq!(output.status.code(), Some(1));
    let changes: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    let removal = changes
        .iter()
        .find(|c| c["function"] == "pets.delete")
        .unwrap();
    assert_eq!(removal["severity"], "breaking");
    assert_eq!(removal["kind"], "function-removed");
    assert_eq!(removal["key"], "pets.delete [function-removed]");
    assert!(changes
        .iter()
        .filter(|c| c["kind"] == "variant-added"
            && c["location"].as_str().unwrap().starts_with("output"))
        .all(|c| c["severity"] == "warning"));
}

#[test]
fn bad_input_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let acks = dir.path().join("acks.md");
    std::fs::write(
        &acks,
        "- pets.delete [function-removed]: <reason, consumers checked, ticket>\n",
    )
    .unwrap();
    assert_eq!(
        diff(dir.path(), &["--acknowledged", acks.to_str().unwrap()])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        diff(dir.path(), &["--today", "2026-02-30"]).status.code(),
        Some(2)
    );
    assert_eq!(
        diff(dir.path(), &["--warn", "no-such-kind"]).status.code(),
        Some(2)
    );
}

#[test]
fn deprecated_in_requires_the_removal_to_have_been_deprecated_back_then() {
    let dir = tempfile::tempdir().unwrap();
    let sunset = |mut schema: Value| {
        for f in schema["functions"].as_array_mut().unwrap() {
            if f["name"] == "pets.delete" {
                f["sunset"] = json!("2026-11-01");
            }
        }
        schema
    };
    let undeprecated = |mut schema: Value| {
        for f in schema["functions"].as_array_mut().unwrap() {
            if f["name"] == "pets.delete" {
                f.as_object_mut().unwrap().remove("deprecation_note");
            }
        }
        schema
    };
    let old = sunset(demo_schema());
    let mut new = old.clone();
    new["functions"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "pets.delete");
    let write = |name: &str, schema: &Value| {
        let path = dir.path().join(name);
        std::fs::write(&path, schema.to_string()).unwrap();
        path
    };
    let (old_path, new_path) = (write("old.json", &old), write("new.json", &new));
    let run = |snapshot: &Path| {
        Command::new(env!("CARGO_BIN_EXE_reflectapi"))
            .arg("diff")
            .args([&old_path, &new_path])
            .args(["--today", "2026-11-20", "--deprecated-in"])
            .arg(snapshot)
            .output()
            .unwrap()
    };

    let deprecated_then = run(&write("then.json", &demo_schema()));
    assert_eq!(
        deprecated_then.status.code(),
        Some(0),
        "{}",
        stdout(&deprecated_then)
    );

    let not_deprecated_then = run(&write("not-then.json", &undeprecated(demo_schema())));
    assert_eq!(not_deprecated_then.status.code(), Some(1));
    assert!(stdout(&not_deprecated_then)
        .contains("pets.delete: function removed, not deprecated in the earlier schema"));

    assert_eq!(run(&dir.path().join("missing.json")).status.code(), Some(2));
}
