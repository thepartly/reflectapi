//! The same boundary, natively: a request on stdin, the response on stdout.
use std::io::{Read, Write};

fn main() {
    let mut request = Vec::new();
    std::io::stdin().read_to_end(&mut request).unwrap();
    std::io::stdout()
        .write_all(&reflectapi_codegen_wasm::handle(&request))
        .unwrap();
}
