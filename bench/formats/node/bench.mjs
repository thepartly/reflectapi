// JSON, MessagePack and CBOR in Node: size and decode/encode time on API-shaped payloads.
//   npm install && npm run bench     (npm run dump writes the payloads for ../rust)
import zlib from "node:zlib";
import { isDeepStrictEqual } from "node:util";
import { Packr, Unpackr, isNativeAccelerationEnabled } from "msgpackr";
import * as mp from "@msgpack/msgpack";
import { Encoder as CborEncoder, Decoder as CborDecoder, isNativeAccelerationEnabled as cborNative } from "cbor-x";
import * as cborg from "cborg";

// Deterministic data.
let seed = 42;
const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 2 ** 32);
const int = (n) => Math.floor(rnd() * n);
const pick = (a) => a[int(a.length)];
const words = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu steel brass copper filter bracket sensor valve housing gasket".split(" ");
const text = (n) => Array.from({ length: n }, () => pick(words)).join(" ");
const hex = (n) => Array.from({ length: n }, () => int(16).toString(16)).join("");
const uuid = () => `${hex(8)}-${hex(4)}-7${hex(3)}-${pick("89ab")}${hex(3)}-${hex(12)}`;
const date = () => new Date(1.7e12 + int(3e10)).toISOString();

// Shaped like a reflectapi response: snake_case keys, internally tagged enums,
// decimals as strings, nulls for absent optionals, items + pagination.
function record() {
  return {
    id: uuid(),
    sku: `SKU-${hex(6).toUpperCase()}`,
    name: text(3 + int(5)),
    description: rnd() < 0.3 ? null : text(20 + int(30)),
    price: { amount: (int(100000) / 100).toFixed(2), currency: pick(["NZD", "AUD", "GBP", "USD"]) },
    quantity: int(500),
    rating: Math.round(rnd() * 500) / 100,
    tags: Array.from({ length: int(5) }, () => pick(words)),
    status: pick([{ type: "active" }, { type: "discontinued", since: date() }, { type: "backordered", eta_days: int(30) }]),
    supplier: { id: uuid(), name: text(2), country: pick(["NZ", "AU", "GB", "US"]) },
    dimensions: { width_mm: int(2000), height_mm: int(2000), depth_mm: int(2000), weight_g: int(50000) },
    created_at: date(),
    updated_at: rnd() < 0.5 ? null : date(),
  };
}

const payloads = {
  "list (1,000 records)": { items: Array.from({ length: 1000 }, record), pagination: { next_cursor: hex(40) } },
  "one record": record(),
  "numeric (5,000 points)": {
    items: Array.from({ length: 5000 }, (_, i) => ({ t: 1.7e12 + i * 1000, count: int(1e6), values: Array.from({ length: 8 }, () => rnd() * 1000) })),
  },
  "text (200 docs, ~2 KB each)": {
    items: Array.from({ length: 200 }, () => ({ id: uuid(), title: text(8), body: text(320), created_at: date() })),
  },
};

if (process.env.DUMP) {
  const fs = await import("node:fs");
  const names = ["list", "one", "numeric", "text"];
  fs.mkdirSync(process.env.DUMP, { recursive: true });
  Object.values(payloads).forEach((v, i) => fs.writeFileSync(`${process.env.DUMP}/${names[i]}.json`, JSON.stringify(v)));
  process.exit(0);
}

const plain = new Packr({ useRecords: false }); // standard MessagePack maps: what rmp-serde reads
const plainU = new Unpackr({ useRecords: false });
const records = new Packr({ useRecords: true }); // msgpackr's record extension: not standard MessagePack
const recordsU = new Unpackr({ useRecords: true });
const cbor = new CborEncoder({ useRecords: false }); // standard CBOR maps
const cborD = new CborDecoder({ useRecords: false });
const cborRec = new CborEncoder({ useRecords: true }); // cbor-x's record extension: not standard
const cborRecD = new CborDecoder({ useRecords: true });

let sink = 0;
function time(fn) {
  // Warm up, then batches of ~100 ms; report the median time per call.
  const t0 = performance.now();
  let n = 0;
  while (performance.now() - t0 < 300) { sink ^= !!fn(); n++; }
  const per = Math.max(1, Math.ceil(n / 3));
  const samples = [];
  for (let b = 0; b < 15; b++) {
    const s = process.hrtime.bigint();
    for (let i = 0; i < per; i++) sink ^= !!fn();
    samples.push(Number(process.hrtime.bigint() - s) / 1e6 / per);
  }
  samples.sort((a, b) => a - b);
  return samples[7];
}

const fmtKB = (n) => (n / 1024).toFixed(1);
const fmtT = (ms) => (ms < 0.1 ? `${(ms * 1000).toFixed(1)} µs` : `${ms.toFixed(2)} ms`);
const br = (b) => zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 5 } }).length;
const gz = (b) => zlib.gzipSync(b, { level: 6 }).length;

console.log(`Node ${process.version}, msgpackr native: ${isNativeAccelerationEnabled}, cbor-x native: ${cborNative}\n`);
for (const [name, value] of Object.entries(payloads)) {
  const jsonStr = JSON.stringify(value);
  const json = Buffer.from(jsonStr);
  const mpk = plain.pack(value);
  const mpkOld = Buffer.from(mp.encode(value));
  const rec = records.pack(value);
  const cb = cbor.encode(value);
  const cbRec = cborRec.encode(value);
  const cbg = Buffer.from(cborg.encode(value));

  // Correctness: every decoder gives back the same value.
  for (const back of [plainU.unpack(mpk), mp.decode(mpkOld), recordsU.unpack(rec), cborD.decode(cb), cborRecD.decode(cbRec), cborg.decode(cbg)])
    if (!isDeepStrictEqual(back, value)) throw new Error(`round trip differs: ${name}`);

  const rows = [
    ["JSON", json, () => JSON.parse(json.toString("utf8")), () => Buffer.from(JSON.stringify(value))],
    ["MessagePack, msgpackr", mpk, () => plainU.unpack(mpk), () => plain.pack(value)],
    ["MessagePack, @msgpack/msgpack", mpkOld, () => mp.decode(mpkOld), () => mp.encode(value)],
    ["msgpackr records (non-standard)", rec, () => recordsU.unpack(rec), () => records.pack(value)],
    ["CBOR, cbor-x", cb, () => cborD.decode(cb), () => cbor.encode(value)],
    ["CBOR, cborg (sorts keys)", cbg, () => cborg.decode(cbg), () => cborg.encode(value)],
    ["cbor-x records (non-standard)", cbRec, () => cborRecD.decode(cbRec), () => cborRec.encode(value)],
  ];

  console.log(`## ${name}`);
  console.log("| Format | Raw KB | gzip KB | brotli KB | Decode | Encode |");
  console.log("|---|---:|---:|---:|---:|---:|");
  for (const [label, bytes, decode, encode] of rows) {
    console.log(`| ${label} | ${fmtKB(bytes.length)} | ${fmtKB(gz(bytes))} | ${fmtKB(br(bytes))} | ${fmtT(time(decode))} | ${fmtT(time(encode))} |`);
  }
  console.log(`JSON.parse on an already-decoded string: ${fmtT(time(() => JSON.parse(jsonStr)))}\n`);
}
if (sink === 2) console.log(sink);
