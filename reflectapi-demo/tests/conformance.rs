//! Wire conformance: every generated client, and the OpenAPI spec, agree with
//! serde on the JSON a server sends and accepts.
//!
//! Each case is an echo route, `conformance.<case>`, whose request and
//! response are both the case's type. `write_conformance_fixture` writes to
//! `target/conformance/`:
//!
//! - `samples.json`: for each case, serde's JSON for several values of its
//!   type, and the divergences we know about;
//! - the Python, TypeScript and Rust clients, and `openapi.json`;
//! - `typescript/cases.ts` and `rust/cases.rs`, which call each route.
//!
//! The harnesses then check every sample in every language:
//!
//! - **request**: a value decoded from the sample is sent as the sample;
//! - **response**: the sample, served back, decodes and re-encodes unchanged;
//! - **types** (TypeScript): the sample type-checks as the route's input and
//!   output;
//! - **schema** (OpenAPI): the sample validates against the route's request
//!   and response schemas.
//!
//! The harnesses are `tests/python/test_conformance.py` (Python and OpenAPI),
//! `tests/typescript/run-conformance.sh` and `tests/rust/conformance`.
//!
//! A known divergence is listed in `DIVERGENCES` and expected to fail. When
//! one starts passing, its harness fails until the entry is removed.

use std::path::Path;

pub mod cases {
    use std::collections::BTreeMap;

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(rename_all = "camelCase")]
    pub struct Record {
        pub id: u32,
        pub display_name: String,
        #[serde(rename = "type")]
        pub kind: String,
        pub score: f64,
        pub active: bool,
        pub delta: i64,
        pub note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hint: Option<String>,
        pub tags: Vec<String>,
        pub counts: BTreeMap<String, u32>,
        pub point: Point,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Pair(pub u32, pub String);

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Id(pub u32);

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(transparent)]
    pub struct Name {
        pub inner: String,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Marker;

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(transparent)]
    pub struct Transparent<T> {
        pub inner: T,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Newtype<T>(pub T);

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct GenericWrappers {
        pub transparent: Transparent<Point>,
        pub newtype: Newtype<Point>,
        pub numbers: Transparent<Vec<u32>>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Wrappers {
        pub id: Id,
        pub name: Name,
        pub pair: Pair,
        pub ids: Vec<Id>,
        pub marker: Marker,
        pub boxed: Box<Point>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(rename_all = "snake_case")]
    pub enum Color {
        Red,
        DarkBlue,
        #[serde(rename = "GREEN")]
        Green,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(rename_all = "snake_case")]
    pub enum External {
        Unit,
        Newtype(u32),
        Tuple(u32, String),
        Struct {
            a: u32,
            #[serde(rename = "B")]
            b: Option<String>,
        },
        #[serde(rename = "renamed")]
        Other(Point),
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "type")]
    pub enum Internal {
        Unit,
        Struct {
            a: u32,
            b: Option<String>,
        },
        Newtype(Point),
        #[serde(rename = "renamed")]
        Other {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            c: Option<u32>,
        },
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "t", content = "c")]
    pub enum Adjacent {
        Unit,
        Newtype(String),
        Tuple(u32, u32),
        Struct {
            a: u32,
        },
        #[serde(rename = "renamed")]
        Other(u32),
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(untagged)]
    pub enum Untagged {
        Number(u32),
        Text(String),
        Object { a: u32 },
        List(Vec<u32>),
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Extra {
        pub label: String,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Flattened {
        pub id: u32,
        #[serde(flatten)]
        pub point: Point,
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        pub extra: Option<Extra>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenedEnum {
        pub id: u32,
        #[serde(flatten)]
        pub shape: Internal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Page<T> {
        pub items: Vec<T>,
        pub next: Option<String>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Patch {
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub name: reflectapi::Option<String>,
        #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
        pub point: reflectapi::Option<Point>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Maps {
        pub by_id: BTreeMap<u32, String>,
        pub nested: BTreeMap<String, Vec<Point>>,
        pub optional: Vec<Option<u32>>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Numbers {
        pub big: u64,
        pub negative: i64,
        pub small: i8,
        pub float: f64,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Stamped {
        pub at: chrono::DateTime<chrono::Utc>,
        pub local: chrono::NaiveDateTime,
        pub day: chrono::NaiveDate,
        pub time: chrono::NaiveTime,
        pub id: uuid::Uuid,
    }
}

/// Generates the echo routes and the serde samples from one list, so a case
/// can't have a route without samples or the reverse. Case names have no
/// underscores: the Python client reads `a.b_c` as group `a_b`, method `c`.
macro_rules! conformance_cases {
    ($($case:ident: $ty:ty = [$($sample:expr),+ $(,)?];)+) => {
        fn schema() -> reflectapi::Schema {
            reflectapi::Builder::<()>::new()
                .name("Conformance")
                $(
                    .route(
                        |_: (), input: $ty, _: reflectapi::Empty| async { input },
                        |b| b.name(concat!("conformance.", stringify!($case))),
                    )
                )+
                .build()
                .unwrap()
                .0
        }

        fn samples() -> Vec<(&'static str, Vec<serde_json::Value>)> {
            vec![$((stringify!($case), vec![$(sample::<$ty>($sample)),+])),+]
        }
    };
}

/// Serde's JSON for `value`, which must also deserialize back to itself;
/// otherwise the sample would be testing an ambiguity in serde, not a client.
fn sample<T: serde::Serialize + serde::de::DeserializeOwned>(value: T) -> serde_json::Value {
    let json = serde_json::to_value(&value).unwrap();
    let back: T = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), json);
    json
}

use cases::*;

fn point(x: i32, y: i32) -> Point {
    Point { x, y }
}

conformance_cases! {
    record: Record = [
        Record {
            id: 1,
            display_name: "One".into(),
            kind: "first".into(),
            score: 1.5,
            active: true,
            delta: -3,
            note: None,
            hint: None,
            tags: vec![],
            counts: Default::default(),
            point: point(0, 0),
        },
        Record {
            id: 2,
            display_name: "Two".into(),
            kind: "second".into(),
            score: 0.25,
            active: false,
            delta: 1 << 40,
            note: Some("noted".into()),
            hint: Some("hinted".into()),
            tags: vec!["a".into(), "b".into()],
            counts: [("x".to_string(), 1), ("y".to_string(), 2)].into(),
            point: point(-1, 7),
        },
    ];
    wrappers: Wrappers = [Wrappers {
        id: Id(7),
        name: Name { inner: "seven".into() },
        pair: Pair(3, "three".into()),
        ids: vec![Id(1), Id(2)],
        marker: Marker,
        boxed: Box::new(point(1, 2)),
    }];
    genericwrappers: GenericWrappers = [GenericWrappers {
        transparent: Transparent { inner: point(1, 2) },
        newtype: Newtype(point(3, 4)),
        numbers: Transparent { inner: vec![5, 6] },
    }];
    tuple: Pair = [Pair(1, "one".into())];
    color: Color = [Color::Red, Color::DarkBlue, Color::Green];
    external: External = [
        External::Unit,
        External::Newtype(1),
        External::Tuple(2, "two".into()),
        External::Struct { a: 3, b: None },
        External::Struct { a: 4, b: Some("four".into()) },
        External::Other(point(5, 6)),
    ];
    internal: Internal = [
        Internal::Unit,
        Internal::Struct { a: 1, b: None },
        Internal::Newtype(point(2, 3)),
        Internal::Other { c: None },
        Internal::Other { c: Some(4) },
    ];
    adjacent: Adjacent = [
        Adjacent::Unit,
        Adjacent::Newtype("one".into()),
        Adjacent::Tuple(2, 3),
        Adjacent::Struct { a: 4 },
        Adjacent::Other(5),
    ];
    untagged: Untagged = [
        Untagged::Number(1),
        Untagged::Text("two".into()),
        Untagged::Object { a: 3 },
        Untagged::List(vec![4, 5]),
    ];
    flattened: Flattened = [
        Flattened { id: 1, point: point(2, 3), extra: None },
        Flattened { id: 4, point: point(5, 6), extra: Some(Extra { label: "seven".into() }) },
    ];
    flatenum: FlattenedEnum = [
        FlattenedEnum { id: 1, shape: Internal::Unit },
        FlattenedEnum { id: 2, shape: Internal::Struct { a: 3, b: Some("four".into()) } },
        FlattenedEnum { id: 5, shape: Internal::Newtype(point(6, 7)) },
    ];
    generic: Page<Point> = [
        Page { items: vec![], next: None },
        Page { items: vec![point(1, 2), point(3, 4)], next: Some("cursor".into()) },
    ];
    patch: Patch = [
        Patch { name: reflectapi::Option::Undefined, point: reflectapi::Option::Undefined },
        Patch { name: reflectapi::Option::None, point: reflectapi::Option::None },
        Patch { name: reflectapi::Option::Some("name".into()), point: reflectapi::Option::Some(point(1, 2)) },
    ];
    maps: Maps = [Maps {
        by_id: [(1, "one".to_string()), (20, "twenty".to_string())].into(),
        nested: [("a".to_string(), vec![point(1, 2)])].into(),
        optional: vec![Some(1), None, Some(3)],
    }];
    numbers: Numbers = [
        Numbers { big: 1, negative: -1, small: -128, float: 0.5 },
        Numbers { big: u64::MAX, negative: i64::MIN, small: 127, float: 1e300 },
    ];
    stamped: Stamped = [
        Stamped {
            at: "2026-11-19T10:20:30Z".parse().unwrap(),
            local: "2026-11-19T10:20:30".parse().unwrap(),
            day: "2026-11-19".parse().unwrap(),
            time: "10:20:30".parse().unwrap(),
            id: "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap(),
        },
        // `Utc::now()` has nanoseconds on most platforms, and serde writes them.
        // Python's `time` field is a `str`, so only its datetimes lose them.
        Stamped {
            at: "2026-11-19T10:20:30.123456789Z".parse().unwrap(),
            local: "2026-11-19T10:20:30.123456789".parse().unwrap(),
            day: "2026-11-19".parse().unwrap(),
            time: "10:20:30.123456789".parse().unwrap(),
            id: "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap(),
        },
    ];
}

/// A known disagreement with serde, expected to fail until it's fixed.
struct Divergence {
    case: &'static str,
    /// Every sample of the case when `None`.
    sample: Option<usize>,
    /// `python`, `typescript`, `rust` or `openapi`.
    target: &'static str,
    /// `request` or `response`; for TypeScript also `types-input` or
    /// `types-output`, and for OpenAPI `schema-request` or `schema-response`.
    check: &'static str,
    reason: &'static str,
}

const DIVERGENCES: &[Divergence] = &[
    Divergence {
        case: "numbers",
        sample: Some(1),
        target: "typescript",
        check: "request",
        reason: "64-bit integers are typed `number`, which rounds beyond 2^53",
    },
    Divergence {
        case: "numbers",
        sample: Some(1),
        target: "typescript",
        check: "response",
        reason: "64-bit integers are typed `number`, which rounds beyond 2^53",
    },
];

#[test]
fn write_conformance_fixture() {
    let schema = schema();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/conformance");
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let write = |path: &str, contents: &str| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    };

    let samples = samples();
    let divergences =
        DIVERGENCES
            .iter()
            .map(|d| {
                assert!(
                    samples.iter().any(|(case, values)| *case == d.case
                        && d.sample.is_none_or(|i| i < values.len())),
                    "divergence names no sample: {} {:?}",
                    d.case,
                    d.sample
                );
                serde_json::json!({
                    "case": d.case,
                    "sample": d.sample,
                    "target": d.target,
                    "check": d.check,
                    "reason": d.reason,
                })
            })
            .collect::<Vec<_>>();
    write(
        "samples.json",
        &serde_json::to_string_pretty(&serde_json::json!({
            "cases": samples
                .iter()
                .map(|(case, values)| (case.to_string(), serde_json::json!({
                    "path": format!("/conformance.{case}"),
                    "samples": values,
                })))
                .collect::<serde_json::Map<_, _>>(),
            "divergences": divergences,
        }))
        .unwrap(),
    );
    write(
        "reflectapi.json",
        &serde_json::to_string_pretty(&schema).unwrap(),
    );
    write(
        "openapi.json",
        &reflectapi::codegen::openapi::generate(&schema, &Default::default()).unwrap(),
    );

    for (filename, source) in
        reflectapi::codegen::typescript::generate(schema.clone(), &Default::default()).unwrap()
    {
        write(&format!("typescript/{filename}"), &source);
    }
    write("typescript/cases.ts", &typescript_cases(&samples));

    let mut rust_config = reflectapi::codegen::rust::Config::default();
    rust_config.base_derives(std::collections::BTreeSet::from_iter([
        "Debug".into(),
        "serde::Serialize".into(),
        "serde::Deserialize".into(),
    ]));
    write(
        "rust/generated.rs",
        &reflectapi::codegen::rust::generate(schema.clone(), &rust_config).unwrap(),
    );
    write("rust/cases.rs", &rust_cases(&samples));

    for (filename, source) in reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            package_name: "conformance_client".into(),
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            format: false,
            base_url: None,
        },
    )
    .unwrap()
    {
        write(&format!("python/conformance_client/{filename}"), &source);
    }
}

/// Each sample as a typed constant of its route's input and output types, so
/// `tsc --strict` checks that the generated types admit serde's JSON, and a
/// call per case for `tests/typescript/conformance.ts`. A known `types`
/// divergence gets `@ts-expect-error`, which fails once the sample checks.
fn typescript_cases(samples: &[(&str, Vec<serde_json::Value>)]) -> String {
    let mut out = String::from(
        "// Generated by reflectapi-demo/tests/conformance.rs\n\
         import type { __definition, AsyncResult, Result } from \"./generated\";\n\
         \n\
         export type Api = __definition.Interface[\"conformance\"];\n\
         type In<K extends keyof Api> = Parameters<Api[K]>[0];\n\
         type Out<K extends keyof Api> =\n  \
           Api[K] extends (...args: any[]) => AsyncResult<infer O, any> ? O : never;\n\
         \n\
         export type Case = {\n  \
           name: string;\n  \
           path: string;\n  \
           /** serde's JSON, as text so that no number is rounded before the check. */\n  \
           samples: string[];\n  \
           call: (api: Api, input: any) => Promise<Result<unknown, unknown>>;\n\
         };\n\n",
    );
    for (case, values) in samples {
        for (i, value) in values.iter().enumerate() {
            let json = serde_json::to_string(value).unwrap();
            for (direction, alias, check) in [
                ("input", "In", "types-input"),
                ("output", "Out", "types-output"),
            ] {
                if let Some(d) = DIVERGENCES.iter().find(|d| {
                    d.case == *case
                        && d.target == "typescript"
                        && d.check == check
                        && d.sample.is_none_or(|s| s == i)
                }) {
                    out.push_str(&format!("// {}\n// @ts-expect-error\n", d.reason));
                }
                out.push_str(&format!(
                    "export const {case}_{i}_{direction}: {alias}<\"{case}\"> = {json};\n"
                ));
            }
        }
    }
    out.push_str("\nexport const cases: Case[] = [\n");
    for (case, values) in samples {
        let texts = values
            .iter()
            .map(|v| serde_json::to_string(v).unwrap())
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "  {{\n    name: \"{case}\",\n    path: \"/conformance.{case}\",\n    samples: {},\n    \
             call: (api, input) => api.{case}(input, {{}}),\n  }},\n",
            serde_json::to_string(&texts).unwrap()
        ));
    }
    out.push_str("];\n");
    out
}

/// One `#[test]` per case for `tests/rust/conformance`, calling the route
/// with each sample through `check`.
fn rust_cases(samples: &[(&str, Vec<serde_json::Value>)]) -> String {
    let mut out = String::from("// Generated by reflectapi-demo/tests/conformance.rs\n");
    for (case, _) in samples {
        out.push_str(&format!(
            "\n#[test]\nfn {case}() {{\n    \
             check(\"{case}\", |api, input| async move {{\n        \
             api.conformance.{case}(input, reflectapi::Empty {{}}).await\n    \
             }});\n}}\n"
        ));
    }
    out
}

#[test]
fn every_case_has_a_route() {
    let schema = schema();
    for (case, _) in samples() {
        let name = format!("conformance.{case}");
        assert!(
            schema.functions().any(|f| f.name == name),
            "no route for {case}"
        );
    }
}
