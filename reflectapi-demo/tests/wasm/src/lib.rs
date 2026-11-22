//! Codegen behind a bytes-in, bytes-out boundary.
//!
//! A host passes a JSON request and gets a JSON response:
//!
//! ```json
//! {"version": 1, "target": "typescript", "schema": {...}, "options": {"format": false}}
//! {"version": 1, "files": {"generated.ts": "...", ...}}
//! {"version": 1, "error": "..."}
//! ```
//!
//! Built for `wasm32-unknown-unknown`, the module imports nothing, so
//! codegen can't reach a filesystem, a process, the environment or a clock.
//! `check.mjs` runs it under Node and compares its output with the native
//! build's (`src/main.rs`).
//!
//! The wasm ABI: `alloc(len) -> ptr` reserves the request, `generate(ptr,
//! len) -> (ptr << 32) | len` returns the response, and `dealloc(ptr, len)`
//! frees either.

use std::collections::BTreeMap;

pub const VERSION: u32 = 1;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    target: Target,
    schema: reflectapi::Schema,
    #[serde(default)]
    options: Options,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Target {
    Typescript,
    Rust,
    Python,
    Openapi,
}

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    #[serde(default)]
    format: bool,
    #[serde(default)]
    typecheck: bool,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Response {
    Files {
        version: u32,
        files: BTreeMap<String, String>,
    },
    Error {
        version: u32,
        error: String,
    },
}

fn generate_files(request: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let request: Request = serde_json::from_slice(request).map_err(|e| e.to_string())?;
    if request.version != VERSION {
        return Err(format!(
            "request version {} isn't supported; this build supports {VERSION}",
            request.version
        ));
    }
    let Options { format, typecheck } = request.options;
    let schema = request.schema;
    let files = match request.target {
        Target::Typescript => reflectapi::codegen::typescript::generate(
            schema,
            reflectapi::codegen::typescript::Config::default()
                .format(format)
                .typecheck(typecheck),
        ),
        Target::Rust => reflectapi::codegen::rust::generate(
            schema,
            reflectapi::codegen::rust::Config::default()
                .format(format)
                .typecheck(typecheck),
        )
        .map(|source| [("generated.rs".to_owned(), source)].into()),
        Target::Python => {
            if typecheck {
                return Err("typecheck isn't available for Python".to_owned());
            }
            reflectapi::codegen::python::generate_files(
                schema,
                reflectapi::codegen::python::Config::default()
                    .format(format),
            )
        }
        Target::Openapi => reflectapi::codegen::openapi::generate(&schema, &Default::default())
            .map(|spec| [("openapi.json".to_owned(), spec)].into()),
    };
    files.map_err(|e| format!("{e:#}"))
}

/// Answers one JSON request with one JSON response. Never panics on bad input.
pub fn handle(request: &[u8]) -> Vec<u8> {
    let response = match generate_files(request) {
        Ok(files) => Response::Files {
            version: VERSION,
            files,
        },
        Err(error) => Response::Error {
            version: VERSION,
            error,
        },
    };
    serde_json::to_vec(&response).expect("a response serializes")
}

/// Reserves `len` bytes for a request; the host frees them with `dealloc`.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// # Safety
/// `ptr` and `len` must come from `alloc` or `generate`, and not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

/// # Safety
/// `ptr` and `len` must describe a request written into memory from `alloc`.
#[no_mangle]
pub unsafe extern "C" fn generate(ptr: *const u8, len: usize) -> u64 {
    let response = handle(std::slice::from_raw_parts(ptr, len)).into_boxed_slice();
    let len = response.len() as u64;
    let ptr = Box::into_raw(response) as *mut u8 as usize as u64;
    (ptr << 32) | len
}
