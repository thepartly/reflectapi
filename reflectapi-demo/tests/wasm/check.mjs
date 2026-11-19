// Runs codegen inside wasm and checks the bytes-in, bytes-out contract:
//
// - the module imports nothing, so it can't reach a filesystem, a process,
//   the environment or a clock;
// - for each schema and target, its files equal the native build's;
// - a step the host must do (formatting, type-checking) is an error that
//   says so, never a trap; Python falls back to its basic formatting.
//
// usage: node check.mjs <module.wasm> <native binary> <schema.json>...
import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const [wasmPath, nativePath, ...schemaPaths] = process.argv.slice(2);
const module = new WebAssembly.Module(readFileSync(wasmPath));
assert.deepEqual(WebAssembly.Module.imports(module), [], "the codegen module imports nothing");

function inWasm(request) {
  // A fresh instance per call, so a trap in one can't hide behind another.
  const { exports } = new WebAssembly.Instance(module, {});
  const bytes = new TextEncoder().encode(JSON.stringify(request));
  const ptr = exports.alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  const packed = exports.generate(ptr, bytes.length);
  exports.dealloc(ptr, bytes.length);
  const outPtr = Number(packed >> 32n);
  const outLen = Number(packed & 0xffffffffn);
  const text = new TextDecoder().decode(new Uint8Array(exports.memory.buffer, outPtr, outLen));
  exports.dealloc(outPtr, outLen);
  return JSON.parse(text);
}

function natively(request) {
  const out = spawnSync(nativePath, { input: JSON.stringify(request), maxBuffer: 1 << 28 });
  assert.equal(out.status, 0, out.stderr.toString());
  return JSON.parse(out.stdout.toString());
}

let checks = 0;
for (const schemaPath of schemaPaths) {
  const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
  for (const target of ["typescript", "rust", "python", "openapi"]) {
    const request = { version: 1, target, schema, options: {} };
    const wasm = inWasm(request);
    assert.ok(wasm.files, `${schemaPath} ${target}: ${wasm.error}`);
    assert.deepEqual(wasm, natively(request), `${schemaPath} ${target}: wasm output differs from native`);
    checks++;
  }
  for (const target of ["typescript", "rust"]) {
    for (const option of ["format", "typecheck"]) {
      const response = inWasm({ version: 1, target, schema, options: { [option]: true } });
      assert.match(response.error ?? "", /host/, `${target} with ${option}: ${JSON.stringify(response).slice(0, 200)}`);
      checks++;
    }
  }
  const python = inWasm({ version: 1, target: "python", schema, options: { format: true } });
  assert.ok(python.files, `python with format: ${python.error}`);
  checks++;
}

const bad = inWasm({ version: 1, target: "cobol", schema: {} });
assert.match(bad.error, /unknown variant/);
assert.match(inWasm({ version: 2, target: "rust", schema: JSON.parse(readFileSync(schemaPaths[0], "utf8")) }).error, /version 2/);
console.log(`wasm codegen: ${checks + 2} checks passed`);
