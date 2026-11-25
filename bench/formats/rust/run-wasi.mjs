// node run-wasi.mjs <file.wasm>: runs a WASI binary with the current directory preopened.
import { WASI } from "node:wasi";
import { readFile } from "node:fs/promises";
const wasi = new WASI({ version: "preview1", args: [process.argv[2]], preopens: { ".": "." }, returnOnExit: true });
const { instance } = await WebAssembly.instantiate(await readFile(process.argv[2]), wasi.getImportObject());
process.exitCode = wasi.start(instance);
